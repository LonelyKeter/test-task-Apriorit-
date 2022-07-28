use std::{
    env, fs,
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
};

fn main() -> io::Result<()> {
    let args = Args::parse().expect("Invalid args.");
    let to_inspect = traverse(&args.root)?;
    let results = search(to_inspect, &args.pattern);
    output_results(results);

    Ok(())
}

struct Args {
    root: PathBuf,
    pattern: Box<[u8]>,
}

impl Args {
    fn parse() -> Option<Self> {
        let args = env::args_os().collect::<Vec<_>>();

        if args.len() != 3 {
            None
        } else {
            Some(Self {
                root: PathBuf::from(args[1].as_os_str()),
                pattern: args[2].to_str()?.as_bytes().to_vec().into_boxed_slice(),
            })
        }
    }
}

fn traverse(root: &Path) -> io::Result<Vec<PathBuf>> {
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }

    let mut files = Vec::new();
    let mut nested_dirs = vec![root.to_path_buf()];

    while let Some(current_dir) = nested_dirs.pop() {
        for entry in fs::read_dir(current_dir)?.filter_map(Result::ok) {
            let file_type = entry.file_type()?;

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

fn inspect_file(path: &Path, pattern: &[u8]) -> io::Result<bool> {
    let buf_len = MIN_BUFF_LEN.max(pattern.len() + 1);

    let mut reader = BufReader::new(fs::File::open(path)?);
    let mut search_buf = vec![0u8; buf_len];

    let mut start_index = 0;
    let tail_len = buf_len - pattern.len() + 1;

    loop {
        let len = reader.read(&mut search_buf[start_index..])?;
        if len == 0 {
            break;
        }

        for window in search_buf[..].windows(pattern.len()) {
            if window == pattern {
                return Ok(true);
            }
        }

        search_buf.copy_within(tail_len.., 0);
        start_index = pattern.len() - 1;
    }

    Ok(false)
}

fn search(paths: Vec<PathBuf>, pattern: &[u8]) -> Vec<PathBuf> {
    let mut matches = Vec::new();

    for path in paths {
        if let Ok(true) = inspect_file(&path, pattern) {
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
