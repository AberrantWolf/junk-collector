use collector_app::Catalog;
use collector_core::CaptureId;
use collector_store::Store;
use std::path::Path;

fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let source = arguments.next().ok_or_else(|| anyhow::anyhow!("source"))?;
    let destination = arguments
        .next()
        .ok_or_else(|| anyhow::anyhow!("destination"))?;
    if arguments.next().is_some() || Path::new(&destination).exists() {
        anyhow::bail!("usage: materialize_wave1_fixture SOURCE NEW_DESTINATION");
    }
    let archive = Path::new(&destination).join("archive");
    let private = Path::new(&destination).join("temporary-private-state");
    copy_tree(Path::new(&source), &archive)?;
    {
        let store = Store::open(&archive, &private)?;
        let capture_id: CaptureId = "018f0000-0000-7000-8000-000000000050".parse()?;
        let mut capture = Catalog::capture(&store, capture_id)?;
        let expected = capture.revision;
        capture.association.carrier_id = None;
        Catalog::save_capture(&store, capture, expected)?;
    }
    std::fs::remove_dir_all(private)?;
    std::fs::remove_file(archive.join(".writer.lock"))?;
    std::fs::remove_dir(archive.join(".staging"))?;
    Ok(())
}
