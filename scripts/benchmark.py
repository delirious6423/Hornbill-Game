"""Repeat a fixed story test with fresh worker processes; never downloads models."""
import argparse
import csv
import json
import os
from pathlib import Path
import subprocess
import time

ACTIONS = [
    "I inspect the radio and ask Mira what she recognizes about the transmission.",
    "I ask Mira to explain her reasoning while I look for a practical way into the observatory.",
    "I take the least destructive available route toward the source of the signal, keeping Mira informed.",
    "I ask Mira what we have learned, what remains unexplained, and what we should do next.",
]


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profiles",nargs="+",choices=["12b","e4b","gguf"],default=["12b"])
    parser.add_argument("--turns",type=int,choices=range(1,5),default=4)
    parser.add_argument("--cpu",action="store_true",help="Explicit GGUF CPU compatibility run; not a Metal benchmark")
    args=parser.parse_args()
    root=Path(os.environ["HORNBILL_ROOT"])
    runtime=Path(os.environ["HORNBILL_RUNTIME"])
    output_dir=root/"data"/f"bench_{time.strftime('%Y%m%d_%H%M%S')}"
    output_dir.mkdir(parents=True,exist_ok=False)
    rows=[]
    profiles=json.loads((root/"workers/gemma/models.json").read_text())
    for profile in args.profiles:
        model=runtime/"models"/("gguf-12b/gemma-4-12b-it-Q4_K_M.gguf" if profile=="gguf" else profiles[profile]["directory"])
        if not model.exists():
            raise SystemExit(f"Model missing: {model}. Download explicitly before benchmarking.")
        save=f"bench_{profile}_{time.time_ns()}"
        base=[str(root/"hornbill"),"--save",save]
        subprocess.run(base+["new","--title",f"Benchmark {profile}"],check=True)
        options=["--backend","gguf" if profile=="gguf" else "mlx","--model",str(model),"--seed","42","--retries","1"]
        if args.cpu and profile=="gguf":
            options += ["--cpu","--timeout-seconds","900"]
        for turn,action in enumerate(ACTIONS[:args.turns],1):
            result=subprocess.run(base+["turn","--action",action]+options)
            if result.returncode:
                print(f"Stopping {profile} after turn {turn} failed; preserving its audit.")
                break
        report=output_dir/f"{profile}.json"
        subprocess.run(base+["export","--output",str(report)],check=True)
        data=json.loads(report.read_text())
        for attempt in data["attempts"]:
            metrics=attempt.get("metadata") or {}
            rows.append({"profile":profile,"turn":attempt["base_turn"]+1,"attempt":attempt["attempt"],
                "status":attempt["status"],"error":attempt.get("error"),
                **{k:metrics.get(k) for k in ["device_requested","load_ms","ttft_ms","prompt_tokens","generation_tokens","generation_tps","peak_rss_bytes","mlx_peak_bytes","worker_total_ms","swap_before_bytes","swap_after_bytes"]}})
    if rows:
        with (output_dir/"metrics.csv").open("w",newline="") as file:
            writer=csv.DictWriter(file,fieldnames=list(rows[0]));writer.writeheader();writer.writerows(rows)
    print(f"Reports: {output_dir}")
    print("Read the generated scenes with docs/BENCHMARKS.md. Valid JSON alone is not a story-quality score.")


if __name__=="__main__":
    main()
