use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use hornbill::{
    database::Database,
    inference::{DemoBackend, GenerationSettings, StoryBackend, process::ProcessBackend},
    models::TurnOutput,
    state::GameState,
    story_engine::Engine,
};
use std::{
    io::{self, Write},
    path::PathBuf,
    time::Duration,
};

#[derive(Parser)]
#[command(version, about = "Project Hornbill — fully local interactive stories")]
struct Cli {
    #[arg(long, global = true, default_value = "main")]
    save: String,
    #[arg(long, global = true, env = "HORNBILL_DATA_DIR", default_value = "data")]
    data_dir: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Open Hornbill's private local graphical interface.
    Ui {
        #[arg(long, default_value_t = 0)]
        port: u16,
        #[arg(long)]
        no_open: bool,
    },
    /// Create a save without loading an AI model. Existing saves are never replaced.
    New {
        #[arg(long, default_value = "Hornbill Observatory")]
        title: String,
        /// Customize the story premise within the starting world.
        #[arg(long)]
        premise: Option<String>,
        /// Start from an edited complete world.json file instead.
        #[arg(long)]
        world: Option<PathBuf>,
    },
    /// Resume a story; enter actions or choice numbers.
    Play(RunArgs),
    /// Generate and save one turn, then exit.
    Turn {
        #[arg(long)]
        action: String,
        #[command(flatten)]
        run: RunArgs,
    },
    /// Print canonical saved state as JSON.
    Inspect,
    /// Illustrate the current saved scene with the isolated local Qwen worker.
    Illustrate {
        #[arg(long, default_value_t = 512)]
        width: u32,
        #[arg(long, default_value_t = 768)]
        height: u32,
        #[arg(long, default_value_t = 6)]
        steps: u32,
        #[arg(long, default_value_t = 42)]
        seed: u32,
    },
    /// Export prompts, responses, attempts, metrics and state snapshots.
    Export {
        #[arg(long)]
        output: PathBuf,
    },
    /// Print the model output JSON Schema.
    Schema,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum BackendKind {
    Mlx,
    Gguf,
    Demo,
}

#[derive(Args)]
struct RunArgs {
    #[arg(long, value_enum, default_value = "mlx")]
    backend: BackendKind,
    /// Local MLX directory or GGUF file. Inference never downloads models.
    #[arg(long, env = "HORNBILL_MODEL")]
    model: Option<PathBuf>,
    #[arg(long, env = "HORNBILL_PYTHON", default_value = "python3")]
    python: PathBuf,
    #[arg(long, env = "HORNBILL_LLAMA_BINARY", default_value = "llama-server")]
    llama_binary: PathBuf,
    /// GGUF only: explicitly run on CPU (for compatibility testing).
    #[arg(long)]
    cpu: bool,
    #[arg(long, default_value_t = 360)]
    timeout_seconds: u64,
    #[arg(long, default_value_t = 1)]
    retries: u32,
    #[arg(long, default_value_t = 1536)]
    max_tokens: usize,
    #[arg(long, default_value_t = 6144)]
    context_tokens: usize,
    #[arg(long, default_value_t = 0.6)]
    temperature: f32,
    #[arg(long, default_value_t = 42)]
    seed: u32,
    #[arg(long, default_value_t = 10)]
    memory_gib: u64,
}

impl RunArgs {
    fn settings(&self) -> GenerationSettings {
        GenerationSettings {
            max_tokens: self.max_tokens,
            context_tokens: self.context_tokens,
            temperature: self.temperature,
            seed: self.seed,
            memory_limit_bytes: self.memory_gib.saturating_mul(1024_u64.pow(3)),
        }
    }
}

fn show(output: &TurnOutput, state: &GameState) {
    println!(
        "\n{} · {} · Turn {}\n",
        state.locations[&output.scene.location].name, output.scene.time, state.turn
    );
    println!("{}\n", output.narration);
    for line in &output.dialogue {
        println!("{}: {}", state.characters[&line.speaker].name, line.text);
    }
    println!();
    for (i, choice) in output.choices.iter().enumerate() {
        println!("  {}. {}", i + 1, choice);
    }
    println!();
}

async fn run_loop(
    mut engine: Engine,
    save: &str,
    args: &RunArgs,
    backend: &mut impl StoryBackend,
    action: Option<String>,
) -> Result<()> {
    if let Some(action) = action {
        let (out, reply) = engine
            .advance(save, &action, backend, args.settings())
            .await?;
        show(&out, &engine.database.load(save)?);
        eprintln!("Saved. Metrics: {}", compact_metrics(&reply.metadata));
        return Ok(());
    }
    println!("Hornbill — save '{save}'. Type an action, a choice number, /state or /quit.");
    if let Some(last) = engine.database.latest_turn(save)? {
        show(&last, &engine.database.load(save)?);
    } else {
        println!("\n{}\n", engine.database.load(save)?.summary);
    }
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match line {
            "/quit" | "/exit" => break,
            "/help" => {
                println!(
                    "Enter an action or a displayed choice number. /state shows your save; /quit exits. Ctrl-C during inference cancels the turn."
                );
                continue;
            }
            "/state" => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&engine.database.load(save)?)?
                );
                continue;
            }
            _ => {}
        }
        let action = if let Ok(index) = line.parse::<usize>() {
            let choices = engine
                .database
                .latest_turn(save)?
                .map(|t| t.choices)
                .unwrap_or_default();
            if index == 0 || index > choices.len() {
                eprintln!("Choose a displayed number, or type an action.");
                continue;
            }
            choices[index - 1].clone()
        } else {
            line.to_owned()
        };
        eprintln!("Writing the next scene…");
        match engine
            .advance(save, &action, backend, args.settings())
            .await
        {
            Ok((out, reply)) => {
                show(&out, &engine.database.load(save)?);
                eprintln!("Saved. Metrics: {}", compact_metrics(&reply.metadata));
            }
            Err(error) => eprintln!("Turn not saved: {error:#}"),
        }
    }
    println!("Story saved. See you next time.");
    Ok(())
}

fn compact_metrics(meta: &serde_json::Value) -> serde_json::Value {
    let keys = [
        "backend",
        "load_ms",
        "ttft_ms",
        "generation_tps",
        "peak_rss_bytes",
        "mlx_peak_bytes",
        "worker_total_ms",
        "attempt_count",
        "swap_before_bytes",
        "swap_after_bytes",
    ];
    keys.into_iter()
        .filter_map(|k| meta.get(k).map(|v| (k.to_owned(), v.clone())))
        .collect::<serde_json::Map<_, _>>()
        .into()
}

async fn execute(engine: Engine, save: &str, args: RunArgs, action: Option<String>) -> Result<()> {
    if matches!(args.backend, BackendKind::Demo) {
        eprintln!("SCRIPTED DEMO: no AI model is being used.");
        return run_loop(engine, save, &args, &mut DemoBackend, action).await;
    }
    let root = std::env::var_os("HORNBILL_ROOT")
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let model = args.model.as_ref().context(
        "no local model configured; run ./scripts/model.sh download 12b, or pass --model PATH",
    )?;
    if !model.exists() {
        bail!("model path does not exist: {}", model.display());
    }
    let script = match args.backend {
        BackendKind::Mlx => "workers/gemma/worker.py",
        BackendKind::Gguf => "workers/gguf/worker.py",
        BackendKind::Demo => unreachable!(),
    };
    let mut worker_args = vec![
        root.join(script).display().to_string(),
        "--model".into(),
        model.display().to_string(),
    ];
    if matches!(args.backend, BackendKind::Gguf) {
        worker_args.extend([
            "--llama-binary".into(),
            args.llama_binary.display().to_string(),
        ]);
        if args.cpu {
            worker_args.push("--cpu".into());
        }
    }
    let mut backend = ProcessBackend {
        executable: args.python.clone(),
        args: worker_args,
        timeout: Duration::from_secs(args.timeout_seconds),
        worker_lock: root.join("data/worker.lock"),
        listen_for_ctrl_c: true,
    };
    run_loop(engine, save, &args, &mut backend, action).await
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Commands::Ui { port, no_open } = &cli.command {
        let root = std::env::var_os("HORNBILL_ROOT")
            .map(PathBuf::from)
            .unwrap_or(std::env::current_dir()?);
        return hornbill::ui::serve(root, cli.data_dir, *port, *no_open).await;
    }
    if matches!(cli.command, Commands::Schema) {
        println!(
            "{}",
            serde_json::to_string_pretty(&hornbill::models::output_schema())?
        );
        return Ok(());
    }
    let database = Database::open(&cli.data_dir.join("hornbill.sqlite3"))?;
    let engine = |retries| Engine {
        database,
        turn_lock: cli.data_dir.join("turn.lock"),
        retries,
    };
    match cli.command {
        Commands::New {
            title,
            premise,
            world,
        } => {
            let mut state = if let Some(path) = world {
                serde_json::from_str::<GameState>(&std::fs::read_to_string(path)?)?
            } else {
                GameState::initial()
            };
            if let Some(premise) = premise {
                state.premise = premise;
            }
            engine(0).database.create_save(&cli.save, &title, &state)?;
            println!(
                "Created save '{}'. Run ./hornbill play --save {}",
                cli.save, cli.save
            );
        }
        Commands::Play(args) => execute(engine(args.retries), &cli.save, args, None).await?,
        Commands::Turn { action, run } => {
            execute(engine(run.retries), &cli.save, run, Some(action)).await?
        }
        Commands::Inspect => println!(
            "{}",
            serde_json::to_string_pretty(&engine(0).database.load(&cli.save)?)?
        ),
        Commands::Illustrate {
            width,
            height,
            steps,
            seed,
        } => {
            let root = std::env::var_os("HORNBILL_ROOT")
                .map(PathBuf::from)
                .unwrap_or(std::env::current_dir()?);
            let runtime = std::env::var_os("HORNBILL_RUNTIME")
                .map(PathBuf::from)
                .unwrap_or_else(|| root.join(".local"));
            let mut backend = hornbill::image_engine::local_backend(&root, &runtime, true);
            let mut db = engine(0).database;
            let mut images = hornbill::image_engine::ImageEngine {
                database: &mut db,
                assets: &cli.data_dir,
                turn_lock: &cli.data_dir.join("turn.lock"),
            };
            eprintln!("Creating the illustration. The story is already saved…");
            let settings = hornbill::image_prompt::ImageSettings {
                width,
                height,
                steps,
                seed,
                ..Default::default()
            };
            let image = images
                .illustrate(
                    &cli.save,
                    &mut backend,
                    hornbill::image_prompt::ImagePolicy::Manual,
                    0.65,
                    settings,
                    true,
                )
                .await?
                .context("image was not generated")?;
            println!(
                "Image: {}",
                cli.data_dir.canonicalize()?.join(&image.path).display()
            );
            eprintln!("Image metrics: {}", image.metadata);
        }
        Commands::Export { output } => {
            let export = engine(0).database.export(&cli.save)?;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&output)
                .context("export path must not already exist")?;
            serde_json::to_writer_pretty(&mut file, &export)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            println!("Exported {}", output.display());
        }
        Commands::Schema | Commands::Ui { .. } => unreachable!(),
    }
    Ok(())
}
