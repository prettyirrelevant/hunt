// Downloads the embedding model and web libraries once, so they compile into the binary.
// hunt downloads the PII model on first use.
use std::{path::Path, process::Command};

const FILES: [(&str, &str); 4] = [
    ("https://huggingface.co/minishlab/potion-base-8M/resolve/main/config.json", "assets/embed/config.json"),
    ("https://huggingface.co/minishlab/potion-base-8M/resolve/main/tokenizer.json", "assets/embed/tokenizer.json"),
    (
        "https://huggingface.co/minishlab/potion-base-8M/resolve/main/model.safetensors",
        "assets/embed/model.safetensors",
    ),
    ("https://cdnjs.cloudflare.com/ajax/libs/echarts/5.5.0/echarts.min.js", "assets/public/vendor/echarts.min.js"),
];

fn main() {
    for (url, target) in FILES {
        let path = Path::new(target);
        println!("cargo:rerun-if-changed={target}");
        if path.exists() {
            continue;
        }
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let status = Command::new("curl")
            .args(["-fsSL", "-o", target, url])
            .status()
            .expect("curl is needed to download bundled assets");
        assert!(status.success(), "failed to download {url}");
    }
}
