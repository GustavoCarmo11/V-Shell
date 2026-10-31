

use std::{
    env,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use amane::Service;

use crate::theme::Theme;

pub struct WallpaperService {
    files: Vec<PathBuf>,
    current: Option<PathBuf>,
    state_file: PathBuf,
}

impl Service for WallpaperService {
    fn new() -> Self {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        let directory = env::var_os("CAELESTIA_RS_WALLPAPER_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("Pictures/Wallpapers"));

        let state_home = env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/state"));

        let state_file = state_home
            .join("caelestia-rs")
            .join("wallpaper");

        let mut files: Vec<PathBuf> = match fs::read_dir(&directory) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file() && supported_image(path)
                })
                .collect(),

            Err(error) => {
                eprintln!(
                    "Wallpaper: erro lendo {:?}: {}",
                    directory,
                    error
                );

                Vec::new()
            }
        };

        files.sort();

        println!(
            "Wallpaper: {} imagens encontradas",
            files.len()
        );

        let current = fs::read_to_string(&state_file)
            .ok()
            .map(|path| PathBuf::from(path.trim()))
            .filter(|path| files.contains(path));

        Self {
            files,
            current,
            state_file,
        }
    }

    fn listen() {}
}

impl WallpaperService {
    pub fn current_path(&self) -> Option<PathBuf> {
        self.current.clone()
    }

    pub fn next() -> Option<PathBuf> {
        // Selecionamos e persistimos a imagem.
        // Liberamos o lock antes de executar o worker.
        let selected = {
            let mut service = Self::write();

            if service.files.is_empty() {
                eprintln!("Wallpaper: nenhum arquivo encontrado");
                return None;
            }

            let current_index = service.current.as_ref().and_then(
                |current| {
                    service.files.iter().position(|path| path == current)
                },
            );

            let next_index = current_index
                .map(|index| (index + 1) % service.files.len())
                .unwrap_or(0);

            let selected = service.files[next_index].clone();

            service.current = Some(selected.clone());
            service.save(&selected);

            selected
        };

        println!("Wallpaper selecionado: {}", selected.display());

        // Gera a nova paleta.
        match generate_palette(&selected) {
            Ok(()) => {
                // Atualiza as cores sem alterar Light/Dark.
                Theme::reload();
            }

            Err(error) => {
                eprintln!(
                    "Wallpaper aplicado, mas não foi possível gerar \
                     a paleta: {error}"
                );
            }
        }

        Some(selected)
    }

    fn save(&self, path: &Path) {
        if let Some(parent) = self.state_file.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                eprintln!("Wallpaper: erro criando diretório: {error}");
                return;
            }
        }

        if let Err(error) = fs::write(
            &self.state_file,
            path.to_string_lossy().as_bytes(),
        ) {
            eprintln!("Wallpaper: erro salvando seleção: {error}");
        }
    }
}

// --------------------------------------------------
// COLOR WORKER
// --------------------------------------------------

fn generate_palette(path: &Path) -> Result<(), String> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let worker = env::var_os("CAELESTIA_RS_COLOR_WORKER")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            home.join(".cargo/bin/caelestia-color-worker")
        });

    if !worker.is_file() {
        return Err(format!(
            "Worker não encontrado em {:?}. \
             Execute cargo install --path color-worker",
            worker
        ));
    }

    let cache_home = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".cache"));

    let palette_file = cache_home
        .join("caelestia-rs")
        .join("palette.txt");

    if let Some(parent) = palette_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| error.to_string())?;
    }

    let output = Command::new(&worker)
        .arg(path)
        .arg(&palette_file)
        .output()
        .map_err(|error| {
            format!("Não foi possível executar o worker: {error}")
        })?;

    if !output.status.success() {
        return Err(format!(
            "Worker falhou.\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ));
    }

    println!(
        "Wallpaper: nova paleta salva em {}",
        palette_file.display()
    );

    Ok(())
}

// --------------------------------------------------
// IMAGE FILTER
// --------------------------------------------------

fn supported_image(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "webp")
    )
}
