"""One-shot GGUF adapter. A private loopback llama-server lives for one request.

Uses the GGUF's native tokenizer/template and llama.cpp JSON-schema decoding.
Rust still speaks only stdin/stdout JSON and reaps the complete process group.
"""
import argparse
import json
import os
from pathlib import Path
import resource
import secrets
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request

sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"gemma"))
from worker import require_downloaded, swap_bytes, validate_request


def generate(args, request, meta):
    settings=validate_request(request)
    if not args.model.is_file() or args.model.suffix.lower()!=".gguf":
        raise ValueError("--model must be an existing local GGUF file")
    binary=shutil.which(str(args.llama_binary))
    if not binary:
        raise ValueError("llama-server is missing; run ./scripts/gguf.sh setup or pass --llama-binary")
    require_downloaded(args.model)
    executable = Path(binary).resolve()
    require_downloaded(executable)
    # The macOS release loads adjacent dylibs even for --version. Detect iCloud
    # placeholders before the dynamic loader can block trying to hydrate them.
    for library in executable.parent.glob("*.dylib"):
        require_downloaded(library)
    # No external endpoint and no proxy inheritance; this server serves one worker.
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1",0))
        port=reservation.getsockname()[1]
    api_key=secrets.token_hex(24)
    opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def http(path, body=None, timeout=10):
        data=None if body is None else json.dumps(body).encode()
        req=urllib.request.Request(f"http://127.0.0.1:{port}{path}",data=data,
                headers={"Content-Type":"application/json","Authorization":f"Bearer {api_key}"})
        try:
            with opener.open(req,timeout=timeout) as response:
                payload=response.read(512*1024+1)
        except urllib.error.HTTPError as error:
            detail=error.read(6000).decode(errors="replace")
            raise RuntimeError(f"llama.cpp HTTP {error.code}: {detail}") from error
        if len(payload)>512*1024:
            raise ValueError("llama.cpp response exceeds 512 KiB")
        return json.loads(payload)

    command=[binary,"--model",str(args.model.resolve()),"--host","127.0.0.1","--port",str(port),
             "--api-key",api_key,"--ctx-size",str(settings["context_tokens"]),"--parallel","1",
             "--batch-size","256","--ubatch-size","128","--no-warmup","--no-context-shift",
             "--jinja","--reasoning","off","--n-gpu-layers","0" if args.cpu else "all"]
    if args.cpu:
        command += ["--device","none","--no-kv-offload"]
    meta.update({"backend":"llama.cpp","model_path":str(args.model.resolve()),"device_requested":"cpu" if args.cpu else "gpu",
                 "settings":settings,"runtime_version":subprocess.check_output([binary,"--version"],stderr=subprocess.STDOUT,text=True,timeout=10).strip()})
    manifest=args.model.with_suffix(".hornbill.json")
    if manifest.exists():
        meta["model"]=json.loads(manifest.read_text())
    with tempfile.TemporaryFile() as logs:
        started=time.perf_counter()
        process=subprocess.Popen(command,stdin=subprocess.DEVNULL,stdout=logs,stderr=logs)
        stop=threading.Event()
        violations=[]
        sampled_peak=[None]

        def guard():
            while not stop.wait(0.5):
                try:
                    rss=int(subprocess.check_output(["ps","-o","rss=","-p",str(process.pid)],timeout=2,stderr=subprocess.DEVNULL).strip())*1024
                    sampled_peak[0]=max(sampled_peak[0] or 0,rss)
                    if rss>settings["memory_limit_bytes"]:
                        violations.append("llama.cpp exceeded the configured sampled RSS budget")
                        process.terminate()
                        return
                except (ValueError,OSError,subprocess.SubprocessError):
                    return
        watcher=threading.Thread(target=guard,daemon=True)
        watcher.start()
        try:
            while True:
                if process.poll() is not None:
                    raise RuntimeError(f"llama.cpp stopped during loading (exit {process.returncode})")
                try:
                    if http("/health",timeout=1).get("status")=="ok":
                        break
                except (OSError,RuntimeError):
                    pass
                if time.perf_counter()-started>180:
                    raise TimeoutError("llama.cpp did not become ready in 180 seconds")
                time.sleep(0.1)
            meta["load_ms"]=(time.perf_counter()-started)*1000
            template=http("/apply-template",{"messages":request["messages"],"add_generation_prompt":True,
                                            "chat_template_kwargs":{"enable_thinking":False}})
            formatted=template["prompt"]
            meta["formatted_prompt"]=formatted
            tokens=http("/tokenize",{"content":formatted,"add_special":False,"parse_special":True})["tokens"]
            meta["prompt_tokens"]=len(tokens)
            if len(tokens)+settings["max_tokens"]>settings["context_tokens"]:
                raise ValueError("Prompt + reserved output exceeds context budget; reduce context or output size")
            generation_start=time.perf_counter()
            # Penalize repeated prose sequences without relaxing JSON grammar.
            # Quote/newline breakers avoid penalizing the repeated schema keys.
            sampling={"dry_multiplier":0.8,"dry_base":1.75,"dry_allowed_length":3,
                      "dry_penalty_last_n":512,"dry_sequence_breakers":["\n",":","\"","*"]}
            meta["sampling"]=sampling
            # /completion consumes the already-rendered native chat template and
            # provides exact timings and raw JSON grammar without chat UI decoration.
            response=http("/completion",{
                "prompt":tokens,"n_predict":settings["max_tokens"],"temperature":settings["temperature"],
                "top_p":0.9,"seed":settings["seed"],"json_schema":request["schema"],
                "cache_prompt":False,"stream":False,"return_tokens":False,**sampling
            },timeout=600)
            if response.get("truncated"):
                raise RuntimeError("llama.cpp truncated context; refusing this turn")
            timing=response.get("timings",{})
            meta.update({"generation_ms":(time.perf_counter()-generation_start)*1000,
                         "ttft_ms":None,"generation_tps":timing.get("predicted_per_second"),
                         "generation_tokens":timing.get("predicted_n"),"prompt_tps":timing.get("prompt_per_second"),
                         "finish_reason":response.get("stop_type"),"timings":timing})
            return response["content"]
        finally:
            stop.set()
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
            process.wait()
            watcher.join(timeout=3)
            logs.seek(0)
            meta["runtime_log"]=logs.read(64*1024).decode(errors="replace").replace(api_key,"[redacted]")
            meta["peak_rss_bytes"]=resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss*(1 if sys.platform=="darwin" else 1024)
            meta["sampled_peak_rss_bytes"]=sampled_peak[0]
            meta["rss_guard_available"]=sampled_peak[0] is not None
            meta["server_exited"]=True
            if violations:
                raise RuntimeError(violations[0])


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model",type=Path,required=True)
    parser.add_argument("--llama-binary",type=Path,default=Path("llama-server"))
    parser.add_argument("--cpu",action="store_true")
    args=parser.parse_args()
    reply={"protocol_version":1,"raw_text":"","metadata":{"swap_before_bytes":swap_bytes()},"error":None}
    try:
        payload=sys.stdin.buffer.read(256*1024+1)
        if len(payload)>256*1024:
            raise ValueError("Request exceeds 256 KiB")
        reply["raw_text"]=generate(args,json.loads(payload),reply["metadata"])
    except Exception as error:
        reply["error"]=f"{type(error).__name__}: {error}"
    reply["metadata"]["swap_after_bytes"]=swap_bytes()
    print(json.dumps(reply,ensure_ascii=False,allow_nan=False),flush=True)


if __name__=="__main__":
    main()
