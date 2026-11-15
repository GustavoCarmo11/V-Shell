use image::imageops::FilterType;

use material_color_utils::{hct::Hct, utils::color_utils::Argb};

use std::{env, error::Error, fs, io, path::PathBuf, process};

use image::DynamicImage;

use material_color_utils::{
    dynamic::variant::Variant, extract_image_colors, theme_from_color, MaterializedScheme,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("color-worker: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {

    let mut args = env::args_os().skip(1);

    // Primeiro argumento: wallpaper original.
    let input = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Informe o caminho do wallpaper",
            )
        })?;

    // Segundo argumento: arquivo de paleta.
    let output = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Informe o caminho da paleta",
            )
        })?;

    // Terceiro argumento: imagem otimizada (opcional).
    let display_output = args.next().map(PathBuf::from);

    // Não permite argumentos extras.
    if args.next().is_some() {
        return Err(
            "Uso: color-worker <imagem> <paleta> [imagem-otimizada]".into()
        );
    }

    if !input.is_file() {
        return Err(format!("Imagem não encontrada: {}", input.display()).into());
    }

    println!("Analisando wallpaper: {}", input.display());

    // Decodifica a imagem.
    let original = image::open(&input)?;
    // --------------------------------------------------
    // OPTIMIZED DISPLAY IMAGE
    // --------------------------------------------------

    if let Some(display_path) = display_output {
        // Redimensiona sem ampliar imagens pequenas,
        // preservando a proporção original.
        let display_image = original.thumbnail(1920, 1200);

        if let Some(parent) = display_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Grava em arquivo temporário para evitar
        // deixar um arquivo incompleto no cache.
        let temporary = display_path.with_extension("tmp.png");

        display_image.save(&temporary)?;
        fs::rename(&temporary, &display_path)?;

        println!(
            "Display image: {}x{}",
            display_image.width(),
            display_image.height()
        );

        println!(
            "Optimized wallpaper saved: {}",
            display_path.display()
        );
    }
    // Reduz a imagem para acelerar a quantização.
    // Mantém a proporção original.
    let thumbnail = original.thumbnail(128, 128);
    // Diagnóstico das cores candidatas.
    let candidates = extract_image_colors(&thumbnail)
        .quantize_max_colors(128)
        .desired_colors(6)
        .call();

    println!("\nCores candidatas:");

    for (index, color) in candidates.iter().enumerate() {
        let hct = Hct::from_argb(*color);

        println!(
            "{}. {} | Hue: {:.1} | Chroma: {:.1} | Tone: {:.1}",
            index + 1,
            color.to_hex(),
            hct.hue(),
            hct.chroma(),
            hct.tone()
        );
    }

    println!();
    // --------------------------------------------------
    // WALLPAPER LIGHT / DARK DETECTION
    // --------------------------------------------------

    // Reduz a thumbnail para 1x1 utilizando Lanczos.
    let average_image = thumbnail.resize_exact(1, 1, FilterType::Lanczos3);

    let pixel = average_image.to_rgb8().get_pixel(0, 0).0;

    // Converte RGB para ARGB.
    let average_argb = Argb(
        0xFF00_0000 | ((pixel[0] as u32) << 16) | ((pixel[1] as u32) << 8) | (pixel[2] as u32),
    );

    // Calcula o Tone perceptual HCT.
    let average_hct = Hct::from_argb(average_argb);
    // Seed acromática, preservando a luminosidade média.
    let gray = (pixel[0] as u16 + pixel[1] as u16 + pixel[2] as u16) / 3;

    let gray = gray as u32;

    let neutral_seed = Argb(0xFF000000 | (gray << 16) | (gray << 8) | gray);

    let suggested_mode = if average_hct.tone() > 60.0 {
        "light"
    } else {
        "dark"
    };

    println!("Wallpaper brightness: HCT Tone {:.1}", average_hct.tone());

    println!("Suggested mode: {suggested_mode}");

    println!("Thumbnail: {}x{}", thumbnail.width(), thumbnail.height());

    // Gera um único tema Material You com as duas paletas.
    // Analisa a intensidade cromática do wallpaper.
    let colorfulness = calc_colourfulness(&thumbnail);

    // Seleciona automaticamente a variante.

    // --------------------------------------------------
    // SOURCE COLOR SELECTION
    // --------------------------------------------------

    // Procura uma cor cromática representativa.
    let selected_seed = candidates.iter().copied().find(|color| {
        let hct = Hct::from_argb(*color);

        hct.chroma() >= 15.0 && (5.0..=95.0).contains(&hct.tone())
    });

    // Escolhe a seed e a variante.
    let (source_seed, variant, variant_name) = if colorfulness < 10.0 {
        (neutral_seed, Variant::Neutral, "neutral")
    } else if let Some(seed) = selected_seed {
        if colorfulness < 20.0 {
            (seed, Variant::Content, "content")
        } else {
            (seed, Variant::TonalSpot, "tonalspot")
        }
    } else {
        (neutral_seed, Variant::Neutral, "neutral")
    };

    println!("Colorfulness: {:.2}", colorfulness);
    println!("Selected variant: {}", variant_name);
    println!("Selected source: {}", source_seed.to_hex());

    // Gera as duas paletas utilizando a seed escolhida.
    let theme = theme_from_color(source_seed).variant(variant).call();

    // Monta nosso formato simples de paleta.
    let mut content = String::new();

    // Informações da extração.
    content.push_str("version=1\n");

    content.push_str(&format!("source={}\n", source_seed.to_hex()));

    content.push_str(&format!("average_tone={:.1}\n", average_hct.tone()));

    content.push_str(&format!("suggested_mode={}\n", suggested_mode));

    content.push_str(&format!("colorfulness={:.2}\n", colorfulness));

    content.push_str(&format!("variant={}\n", variant_name));

    // Paleta Dark.
    append_scheme(&mut content, "dark", &theme.schemes.dark);

    // Paleta Light.
    append_scheme(&mut content, "light", &theme.schemes.light);

    // Cria o diretório de saída.
    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    // Grava primeiro em um arquivo temporário.
    // Depois substitui o arquivo de paleta.
    let temporary = output.with_extension("palette.tmp");

    fs::write(&temporary, content)?;
    fs::rename(&temporary, &output)?;

    println!("Paletas Dark e Light geradas em: {}", output.display());

    Ok(())
}

fn append_scheme(output: &mut String, name: &str, scheme: &MaterializedScheme) {
    let colors = [
        ("background", scheme.background),
        ("surface", scheme.surface),
        ("surface_container", scheme.surface_container),
        ("surface_container_high", scheme.surface_container_high),
        ("surface_container_highest", scheme.surface_container_highest),
        ("surface_container_low", scheme.surface_container_low),
        ("primary", scheme.primary),
        ("on_primary", scheme.on_primary),
        ("primary_container", scheme.primary_container),
        ("on_primary_container", scheme.on_primary_container),
        ("secondary", scheme.secondary),
        ("tertiary", scheme.tertiary),
        ("on_surface", scheme.on_surface),
        ("on_surface_variant", scheme.on_surface_variant),
        ("outline", scheme.outline),
        ("outline_variant", scheme.outline_variant),
        ("error", scheme.error),
    ];

    for (role, color) in colors {
        output.push_str(&format!("{name}.{role}={}\n", color.to_hex()));
    }
}

fn calc_colourfulness(image: &DynamicImage) -> f64 {
    let rgb = image.to_rgb8();

    let mut rg_diffs = Vec::new();
    let mut yb_diffs = Vec::new();

    for pixel in rgb.pixels() {
        let r = pixel[0] as f64;
        let g = pixel[1] as f64;
        let b = pixel[2] as f64;

        let rg = (r - g).abs();
        let yb = (0.5 * (r + g) - b).abs();

        rg_diffs.push(rg);
        yb_diffs.push(yb);
    }

    let mean_rg = mean(&rg_diffs);
    let mean_yb = mean(&yb_diffs);

    let std_rg = stddev(&rg_diffs, mean_rg);
    let std_yb = stddev(&yb_diffs, mean_yb);

    (std_rg.powi(2) + std_yb.powi(2)).sqrt() + 0.3 * (mean_rg.powi(2) + mean_yb.powi(2)).sqrt()
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    values.iter().sum::<f64>() / values.len() as f64
}

fn stddev(values: &[f64], average: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let variance = values
        .iter()
        .map(|value| (value - average).powi(2))
        .sum::<f64>()
        / values.len() as f64;

    variance.sqrt()
}
