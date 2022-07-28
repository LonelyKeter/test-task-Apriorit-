use std::{
    env,
    path::{Path, PathBuf},
    sync::Arc
};

use tokio::{
    io::{self, BufReader, AsyncReadExt}, 
    fs
};

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = Args::parse().expect("Invalid args.");
    let to_inspect = traverse(&args.root).await?;
    let results = search(to_inspect, args.pattern.clone()).await;
    output_results(results);

    Ok(())
}

struct Args {
    root: PathBuf,
    pattern: Arc<[u8]>,
}

impl Args {
    fn parse() -> Option<Self> {
        let args = env::args_os().collect::<Vec<_>>();

        if args.len() != 3 {
            None
        } else {
            Some(Self {
                root: PathBuf::from(args[1].as_os_str()),
                pattern: Arc::from(args[2].to_str()?.as_bytes().to_vec().into_boxed_slice()),
            })
        }
    }
}

async fn traverse(root: &Path) -> io::Result<Vec<PathBuf>> {
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }

    let mut files = Vec::new();
    let mut nested_dirs = vec![root.to_path_buf()];

    while let Some(current_dir) = nested_dirs.pop() {
        let mut entries = fs::read_dir(current_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;

            if file_type.is_file() {
                files.push(entry.path())
            } else if file_type.is_dir() {
                nested_dirs.push(entry.path())
            }
        }
    }

    Ok(files)
}

const MIN_BUFF_LEN: usize = 8 * 1024; //8 KB

async fn inspect_file(path: &Path, pattern: &[u8]) -> io::Result<bool> {
    let buf_len = MIN_BUFF_LEN.max(pattern.len() + 1);

    let mut reader = BufReader::new(fs::File::open(path).await?);
    let mut search_buf = vec![0u8; buf_len];

    let mut start_index = 0;
    let tail_index = buf_len - pattern.len() + 1;

    loop {
        let len = reader.read(&mut search_buf[start_index..]).await?;
        if len == 0 {
            break;
        }

        for window in search_buf[..].windows(pattern.len()) {
            if window == pattern {
                return Ok(true);
            }
        }

        search_buf.copy_within(tail_index.., 0);
        start_index = pattern.len() - 1;
    }

    Ok(false)
}

async fn search(paths: Vec<PathBuf>, pattern:Arc<[u8]>) -> Vec<PathBuf> {
    let mut handles = Vec::with_capacity(paths.len());
    
    for path in paths {
        let pattern = pattern.clone();
        let handle = tokio::spawn(async move {
            if let Ok(true) = inspect_file(&path, &pattern).await {
                Some(path)
            } else {
                None
            } 
        });
        handles.push(handle);
    }

    let mut matches = vec![];
    for handle in handles {
        if let Some(path) = handle.await.unwrap() {
            matches.push(path);
        }
    }

    matches
}

fn output_results(results: Vec<PathBuf>) {
    for path in results {
        println!("{path:?}");
    }
}
