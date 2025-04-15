#[cfg(test)]
mod tests {
    use dummy_markdown::parser;

    const DOC: &'static str = r#"
### **Day 1: Understand Nix & NixOS Basics**
- *Read the [Nix Pills](https://nixos.org/guides/nix-pills/)* (Chapters 1-4) to understand:

- The Nix language basics (functions, sets, lazy evaluation)
    - The Nix store (`/nix/store`)
    - Hello world

- Derivations (how packages are built)

- **Install NixOS** in a VM (VirtualBox, QEMU) or on a spare machine.
- Follow the [NixOS Manual Installation Guide](https://nixos.org/manual/nixos/stable/index.html#sec-installation)
- Run `nixos-rebuild switch` after making changes to `/etc/nixos/configuration.nix`.

![picture](https://nixos.org/manual/nixos/stable/a.png)

```c
int main(void) {
    printf("%s", hello world);
}
```

As with cmark and **cmark-gfm**, Comrak will scrub raw HTML and potentially dangerous links. This change was introduced in Comrak 0.4.0 in support of a safe-by-default posture, and later adopted by our contemporaries. :)

---

Would you like a deeper dive into any of these topics?

2. **Add dependencies to your `Cargo.toml`**:
   ```toml
   [dependencies]
   reqwest = { version = "0.11", features = ["json"] }
   serde = { version = "1.0", features = ["derive"] }
   tokio = { version = "1.0", features = ["full"] }
   ```

| Header 1 | Header 2 | Header 3 |
|----------|----------|----------|
| Cell 1   | Cell 2   | Cell 5   |
| Cell 3   |          | Cell 6   |

"#;

    #[test]
    fn main() {
        let (ui_elems, link_urls) = parser::run(DOC);
        println!("{:#?}", ui_elems);
        println!("{:#?}", link_urls);
    }
}
