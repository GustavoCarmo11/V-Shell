

use std::{
    env,
    error::Error,
    fs,
    io,
    path::PathBuf,
    process,
};

use image::DynamicImage;

use material_color_utils::{
    theme_from_image,
    dynamic::variant::Variant,
    MaterializedScheme,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("color-worker: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);

    let input = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Uso: caelestia-color-worker <imagem> <saida>",
            )
        })?;

    let output = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Informe o arquivo de saída",
            )
        })?;

    if args.next().is_some() {
        return Err(
            "Argumentos extras não são permitidos".into()
        );
    }

    if !input.is_file() {
        return Err(
            format!("Imagem não encontrada: {}", input.display()).into()
        );
    }

    println!("Analisando wallpaper: {}", input.display());

    // Decodifica a imagem.
    let original = image::open(&input)?;

    // Reduz a imagem para acelerar a quantização.
    // Mantém a proporção original.
    let thumbnail = original.thumbnail(128, 128);

    println!(
        "Thumbnail: {}x{}",
        thumbnail.width(),
        thumbnail.height()
    );

    // Gera um único tema Material You com as duas paletas.
    let theme = theme_from_image(&thumbnail)
        .variant(Variant::TonalSpot)
        .quantize_max_colors(128)
        .call()?;

    println!(
        "Cor de origem: {}",
        theme.source_color.to_hex()
    );

    // Monta nosso formato simples de paleta.
    let mut content = String::new();

    content.push_str("version=1\n");
    content.push_str(&format!(
        "source={}\n",
        theme.source_color.to_hex()
    ));

    append_scheme(
        &mut content,
        "dark",
        &theme.schemes.dark,
    );

    append_scheme(
        &mut content,
        "light",
        &theme.schemes.light,
    );

    // Cria o diretório de saída.
    if let Some(parent) = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    // Grava primeiro em um arquivo temporário.
    // Depois substitui o arquivo de paleta.
    let temporary = output.with_extension("palette.tmp");

    fs::write(&temporary, content)?;
    fs::rename(&temporary, &output)?;

    println!(
        "Paletas Dark e Light geradas em: {}",
        output.display()
    );

    Ok(())
}

fn append_scheme(
    output: &mut String,
    name: &str,
    scheme: &MaterializedScheme,
) {
    let colors = [
        ("background", scheme.background),
        ("surface", scheme.surface),
        ("surface_container", scheme.surface_container),
        ("surface_container_high", scheme.surface_container_high),
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
        output.push_str(&format!(
            "{name}.{role}={}\n",
            color.to_hex()
        ));
    }
}
