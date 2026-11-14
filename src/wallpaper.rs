use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use amane::{Animation, Easing, Image, Service};

use crate::theme::Theme;

// --------------------------------------------------
// CONFIGURATION
// --------------------------------------------------

const MAX_CACHE_ENTRIES: usize = 3;
const CROSSFADE_DURATION: Duration = Duration::from_millis(750);

// --------------------------------------------------
// WALLPAPER SERVICE
// --------------------------------------------------

pub struct WallpaperService {
    files: Vec<PathBuf>,
    current: Option<PathBuf>,
    previous: Option<PathBuf>,

    state_file: PathBuf,

    fade: Animation<f32>,
    transition_pending: bool,
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

        let state_file = state_home.join("caelestia-rs").join("wallpaper");

        // Procura imagens no diretório.
        let mut files: Vec<PathBuf> = match fs::read_dir(&directory) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_file() && supported_image(path))
                .collect(),

            Err(error) => {
                eprintln!("Wallpaper: erro lendo {:?}: {}", directory, error);

                Vec::new()
            }
        };

        files.sort();

        println!("Wallpaper: {} imagens encontradas", files.len());

        // Recupera a última imagem selecionada.
        let current = fs::read_to_string(&state_file)
            .ok()
            .map(|path| PathBuf::from(path.trim()))
            .filter(|path| files.contains(path));

        // Atualiza a utilização do cache atual.
        if let Some(path) = current.as_ref() {
            if cache_is_valid(path) {
                mark_cache_used(path);
            }
        }

        Self {
            files,
            current,
            previous: None,
            state_file,

            fade: Animation::new(1.0)
                .duration(CROSSFADE_DURATION)
                .easing(Easing::Out),

            transition_pending: false,
        }
    }

    fn listen() {}
}

// --------------------------------------------------
// WALLPAPER METHODS
// --------------------------------------------------

impl WallpaperService {
    // --------------------------------------------------
    // CROSSFADE
    // --------------------------------------------------

    pub fn transition_snapshot() -> (Option<PathBuf>, Option<PathBuf>, f32) {
        // Captura o estado atual.
        let (current, pending) = {
            let service = Self::read();

            (service.current.clone(), service.transition_pending)
        };

        // Aguarda a imagem nova ser carregada.
        if pending {
            if let Some(path) = current.as_ref() {
                let display_path = display_wallpaper_path(path);

                let image_path = display_path.to_string_lossy().into_owned();

                if Image::loaded(image_path.as_str()) {
                    let mut service = Self::write();

                    if service.transition_pending && service.current.as_ref() == Some(path) {
                        service.transition_pending = false;
                        service.fade.to(1.0);
                    }
                }
            }
        }

        // Obtém os valores interpolados.
        let (previous, current, progress) = {
            let service = Self::read();

            (
                service.previous.clone(),
                service.current.clone(),
                service.fade.value().clamp(0.0, 1.0),
            )
        };

        // Libera a imagem anterior quando a transição termina.
        if progress >= 1.0 && previous.is_some() {
            let mut service = Self::write();

            if service.previous == previous {
                service.previous = None;
            }

            return (None, current.map(|path| display_wallpaper_path(&path)), 1.0);
        }

        (
            previous.map(|path| display_wallpaper_path(&path)),
            current.map(|path| display_wallpaper_path(&path)),
            progress,
        )
    }

    // --------------------------------------------------
    // NEXT WALLPAPER
    // --------------------------------------------------

    pub fn next() -> Option<PathBuf> {
        // Descobre a próxima imagem.
        let selected = {
            let service = Self::read();

            if service.files.is_empty() {
                eprintln!("Wallpaper: nenhum arquivo encontrado");
                return None;
            }

            let current_index = service
                .current
                .as_ref()
                .and_then(|current| service.files.iter().position(|path| path == current));

            let next_index = current_index
                .map(|index| (index + 1) % service.files.len())
                .unwrap_or(0);

            service.files[next_index].clone()
        };

        // Não repete o processamento se só existe uma imagem.
        let unchanged = {
            let service = Self::read();
            service.current.as_ref() == Some(&selected)
        };

        if unchanged {
            if !cache_is_valid(&selected) {
                match generate_palette(&selected) {
                    Ok(()) => {
                        Theme::reload_for_wallpaper();
                        mark_cache_used(&selected);
                    }

                    Err(error) => {
                        eprintln!("Wallpaper: {error}");
                    }
                }
            } else {
                mark_cache_used(&selected);
            }

            return Some(selected);
        }

        println!("Preparando wallpaper: {}", selected.display());

        // Primeiro prepara a imagem e a paleta.
        let palette_generated = match generate_palette(&selected) {
            Ok(()) => {
                println!("Wallpaper: cache preparado.");
                true
            }

            Err(error) => {
                eprintln!("Wallpaper: erro ao preparar: {error}");
                false
            }
        };

        // Atualiza o estado e inicia o crossfade.
        let previous = {
            let mut service = Self::write();

            let previous = service.current.clone();

            service.previous = previous.clone();
            service.current = Some(selected.clone());

            service.fade = Animation::new(0.0)
                .duration(CROSSFADE_DURATION)
                .easing(Easing::Out);

            service.transition_pending = true;

            service.save(&selected);

            previous
        };

        // Atualiza o tema no modo sugerido pelo wallpaper.
        if palette_generated {
            Theme::reload_for_wallpaper();
        }

        // Protege as imagens utilizadas durante o crossfade.
        let mut protected = vec![selected.clone()];

        if let Some(previous) = previous {
            protected.push(previous);
        }

        // Remove os caches menos recentemente utilizados.
        prune_wallpaper_cache(&protected);

        Some(selected)
    }

    // --------------------------------------------------
    // PERSISTENCE
    // --------------------------------------------------

    fn save(&self, path: &Path) {
        if let Some(parent) = self.state_file.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                eprintln!("Wallpaper: erro criando diretório: {error}");

                return;
            }
        }

        let temporary = self.state_file.with_extension("tmp");

        if let Err(error) = fs::write(&temporary, path.to_string_lossy().as_bytes()) {
            eprintln!("Wallpaper: erro salvando seleção: {error}");

            return;
        }

        if let Err(error) = fs::rename(&temporary, &self.state_file) {
            eprintln!("Wallpaper: erro atualizando seleção: {error}");
        }
    }
}

// --------------------------------------------------
// CACHE PATHS
// --------------------------------------------------

fn cache_directory() -> PathBuf {
    let cache_home = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_else(|| PathBuf::from("."));

    cache_home.join("caelestia-rs")
}

fn wallpaper_cache_root() -> PathBuf {
    cache_directory().join("wallpapers")
}

// Cria um identificador estável baseado no caminho original.
fn wallpaper_cache_key(original: &Path) -> String {
    let path = original.to_string_lossy();

    // FNV-1a.
    let mut hash: u64 = 0xcbf29ce484222325;

    for byte in path.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }

    let stem = original.file_stem().unwrap_or_default().to_string_lossy();

    let stem: String = stem
        .chars()
        .take(40)
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect();

    let stem = if stem.is_empty() { "wallpaper" } else { &stem };

    format!("{stem}-{hash:016x}")
}

fn wallpaper_cache_entry(original: &Path) -> PathBuf {
    wallpaper_cache_root().join(wallpaper_cache_key(original))
}

fn optimized_wallpaper_path(original: &Path) -> PathBuf {
    wallpaper_cache_entry(original).join("display.png")
}

fn cached_palette_path(original: &Path) -> PathBuf {
    wallpaper_cache_entry(original).join("palette.txt")
}

// Só utiliza a imagem otimizada quando o cache está válido.
fn display_wallpaper_path(original: &Path) -> PathBuf {
    if cache_is_valid(original) {
        optimized_wallpaper_path(original)
    } else {
        original.to_path_buf()
    }
}

// --------------------------------------------------
// CACHE VALIDATION
// --------------------------------------------------

fn source_signature(original: &Path) -> Result<String, String> {
    let metadata = fs::metadata(original).map_err(|error| error.to_string())?;

    let modified = metadata.modified().map_err(|error| error.to_string())?;

    let modified_ns = modified
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();

    Ok(format!(
        "{}|{}|{}",
        original.display(),
        metadata.len(),
        modified_ns,
    ))
}

fn cache_is_valid(original: &Path) -> bool {
    let entry = wallpaper_cache_entry(original);

    let Ok(signature) = source_signature(original) else {
        return false;
    };

    let Ok(saved_signature) = fs::read_to_string(entry.join("source.meta")) else {
        return false;
    };

    saved_signature == signature
        && optimized_wallpaper_path(original).is_file()
        && cached_palette_path(original).is_file()
}

// --------------------------------------------------
// CACHE ACCESS TIME
// --------------------------------------------------

fn mark_cache_used(original: &Path) {
    let entry = wallpaper_cache_entry(original);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_string();

    if let Err(error) = fs::write(entry.join(".used"), timestamp) {
        eprintln!("Wallpaper cache: erro atualizando utilização: {error}");
    }
}

// --------------------------------------------------
// LRU CACHE CLEANUP
// --------------------------------------------------

fn prune_wallpaper_cache(protected: &[PathBuf]) {
    let root = wallpaper_cache_root();

    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let protected_dirs: HashSet<PathBuf> = protected
        .iter()
        .map(|path| wallpaper_cache_entry(path))
        .collect();

    let mut cached: Vec<(PathBuf, SystemTime)> = Vec::new();

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let used_time = fs::metadata(path.join(".used"))
            .and_then(|metadata| metadata.modified())
            .or_else(|_| fs::metadata(&path).and_then(|metadata| metadata.modified()))
            .unwrap_or(UNIX_EPOCH);

        cached.push((path, used_time));
    }

    // Mais recentemente utilizados primeiro.
    cached.sort_by(|a, b| b.1.cmp(&a.1));

    let existing: HashSet<PathBuf> = cached.iter().map(|(path, _)| path.clone()).collect();

    // Protege o wallpaper atual e o anterior.
    let mut keep: HashSet<PathBuf> = protected_dirs.intersection(&existing).cloned().collect();

    // Completa o cache com os mais recentes.
    for (path, _) in &cached {
        if keep.len() >= MAX_CACHE_ENTRIES {
            break;
        }

        keep.insert(path.clone());
    }

    // Descarta os demais.
    for (path, _) in cached {
        if !keep.contains(&path) {
            match fs::remove_dir_all(&path) {
                Ok(()) => {
                    println!("Wallpaper cache: removido {:?}", path.file_name());
                }

                Err(error) => {
                    eprintln!("Wallpaper cache: erro removendo {:?}: {}", path, error);
                }
            }
        }
    }
}

// --------------------------------------------------
// COLOR WORKER
// --------------------------------------------------

fn generate_palette(original: &Path) -> Result<(), String> {
    let entry = wallpaper_cache_entry(original);

    fs::create_dir_all(&entry).map_err(|error| error.to_string())?;

    let display_file = optimized_wallpaper_path(original);
    let entry_palette = cached_palette_path(original);

    let active_palette = cache_directory().join("palette.txt");

    if let Some(parent) = active_palette.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    // Reaproveita o cache quando a imagem original não mudou.
    if cache_is_valid(original) {
        println!("Wallpaper cache hit: {}", original.display());
    } else {
        println!("Wallpaper cache miss: processando {}", original.display());

        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        let worker = env::var_os("CAELESTIA_RS_COLOR_WORKER")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".cargo/bin/caelestia-color-worker"));

        if !worker.is_file() {
            return Err(format!(
                "Worker não encontrado em {:?}. \
                 Execute cargo install --path color-worker",
                worker
            ));
        }

        // O worker recebe:
        // 1. Imagem original.
        // 2. Paleta individual.
        // 3. Imagem otimizada.
        let output = Command::new(&worker)
            .arg(original)
            .arg(&entry_palette)
            .arg(&display_file)
            .output()
            .map_err(|error| format!("Não foi possível executar o worker: {error}"))?;

        if !output.status.success() {
            return Err(format!(
                "Worker falhou.\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            ));
        }

        // Confirma que os arquivos foram gerados.
        if !display_file.is_file() || !entry_palette.is_file() {
            return Err("Worker terminou sem gerar todos os arquivos".into());
        }

        // Registra a assinatura somente após concluir o processamento.
        let signature = source_signature(original)?;

        fs::write(entry.join("source.meta"), signature).map_err(|error| error.to_string())?;
    }

    // Atualiza atomicamente a paleta ativa.
    let temporary_palette = active_palette.with_extension("tmp");

    fs::copy(&entry_palette, &temporary_palette).map_err(|error| error.to_string())?;

    fs::rename(&temporary_palette, &active_palette).map_err(|error| error.to_string())?;

    mark_cache_used(original);

    println!("Wallpaper otimizado: {}", display_file.display());

    println!("Paleta ativa: {}", active_palette.display());

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
