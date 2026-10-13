use std::collections::BTreeMap;
use std::io::SeekFrom;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::constants::AppConstants;
use crate::models::error::{AppError, AppResult};
use crate::repositories::http_transport::HttpReply;

/// A place that takes a file in numbered parts and joins them later. S3 and Azure each have one.
#[async_trait]
pub trait PartTarget: Send + Sync {
    /// Sends one part. The numbers start at 1.
    async fn send_part(&self, number: u64, bytes: Vec<u8>) -> AppResult<HttpReply>;
}

/// Sends a big file in parts, four parts at the same time. A part that fails is sent again,
/// so a short break in the network does not start the file from zero.
pub struct PartUpload<'a> {
    target: &'a dyn PartTarget,
    file: &'a Path,
    size: u64,
    part_size: u64,
    /// The number of the next part that no worker took yet.
    next: AtomicU64,
    /// Part number -> the `ETag` the server gave for it.
    tags: Mutex<BTreeMap<u64, String>>,
}

impl<'a> PartUpload<'a> {
    /// A small file goes up in one request. That is fewer calls.
    pub fn is_needed(size: u64) -> bool {
        size > AppConstants::UPLOAD_PART_BYTES
    }

    /// The parts get bigger for a very big file, so there are never too many of them.
    pub fn part_size(size: u64) -> u64 {
        AppConstants::UPLOAD_PART_BYTES.max(size.div_ceil(AppConstants::UPLOAD_MAX_PARTS))
    }

    /// Sends the whole file. Returns the tag of each part, in the order of the parts.
    pub async fn run(target: &'a dyn PartTarget, file: &'a Path, size: u64) -> AppResult<Vec<String>> {
        let upload = Self {
            target,
            file,
            size,
            part_size: Self::part_size(size),
            next: AtomicU64::new(1),
            tags: Mutex::new(BTreeMap::new()),
        };
        // ponytail: four workers, written out. Each one holds one part in the memory.
        // Make it a list of workers if the number must come from a setting.
        tokio::try_join!(upload.work(), upload.work(), upload.work(), upload.work())?;
        Ok(upload.tags.into_inner().into_values().collect())
    }

    /// Takes the next free part and sends it, until no part is left.
    async fn work(&self) -> AppResult<()> {
        let mut handle = tokio::fs::File::open(self.file).await?;
        let mut failures = 0;
        let mut number = self.next.fetch_add(1, Ordering::Relaxed);
        loop {
            let start = (number - 1) * self.part_size;
            if start >= self.size {
                return Ok(());
            }
            let length = (self.size - start).min(self.part_size);
            let mut part = vec![0; usize::try_from(length).expect("a part fits in the memory")];
            handle.seek(SeekFrom::Start(start)).await?;
            handle.read_exact(&mut part).await?;

            let sent = self.target.send_part(number, part).await;
            let can_try_again = match &sent {
                Ok(reply) => reply.status >= 500,
                Err(error) => matches!(error, AppError::Network(_)),
            };
            failures += 1;
            if can_try_again && failures < AppConstants::UPLOAD_PART_TRIES {
                tokio::time::sleep(Duration::from_secs(failures)).await;
                continue;
            }
            let reply = sent?;
            reply.check_xml()?;
            self.tags.lock().insert(number, reply.etag.unwrap_or_default());
            failures = 0;
            number = self.next.fetch_add(1, Ordering::Relaxed);
        }
    }
}
