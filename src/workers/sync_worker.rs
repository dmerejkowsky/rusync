use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use entry::Entry;
use fsops;
use fsops::SyncOutcome;
use progress::ProgressMessage;
use sync::SyncOptions;

pub struct SyncWorker {
    input: Receiver<Entry>,
    output: Sender<ProgressMessage>,
    source: PathBuf,
    destination: PathBuf,
}

impl SyncWorker {
    pub fn new(
        source: &Path,
        destination: &Path,
        input: Receiver<Entry>,
        output: Sender<ProgressMessage>,
    ) -> SyncWorker {
        SyncWorker {
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
            input,
            output,
        }
    }

    pub fn start(self, opts: SyncOptions) -> fsops::FSResult<()> {
        for entry in self.input.iter() {
            let sync_outcome = self.sync(&entry, opts)?;
            let progress = ProgressMessage::DoneSyncing(sync_outcome);
            self.output.send(progress).unwrap();
        }
        Ok(())
    }

    fn create_missing_dest_dirs(&self, rel_path: &Path) -> fsops::FSResult<()> {
        let parent_rel_path = rel_path.parent();
        if parent_rel_path.is_none() {
            return Err(fsops::FSError::from_description(&format!(
                "Could not get parent path of {}",
                rel_path.to_string_lossy()
            )));
        }
        let parent_rel_path = parent_rel_path.unwrap();
        let to_create = self.destination.join(parent_rel_path);
        let create_result = fs::create_dir_all(&to_create);
        if let Err(e) = create_result {
            return Err(fsops::FSError::from_io_error(
                e,
                &format!("Could not create {}", &to_create.to_string_lossy()),
            ));
        }
        Ok(())
    }

    fn sync(&self, src_entry: &Entry, opts: SyncOptions) -> fsops::FSResult<SyncOutcome> {
        let rel_path = fsops::get_rel_path(&src_entry.path(), &self.source)?;
        self.create_missing_dest_dirs(&rel_path)?;
        let desc = rel_path.to_string_lossy();

        let dest_path = self.destination.join(&rel_path);
        let dest_entry = Entry::new(&desc, &dest_path);
        let outcome = fsops::sync_entries(&self.output, &src_entry, &dest_entry)?;
        if opts.preserve_permissions {
            fsops::copy_permissions(&src_entry, &dest_entry)?;
        }
        Ok(outcome)
    }
}
