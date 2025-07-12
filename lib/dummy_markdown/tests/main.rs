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

As with cmark and **cmark-gfm**, Comrak **will scrub** raw HTML and potentially dangerous links. This change was introduced in Comrak 0.4.0 in support of a safe-by-default posture, and later adopted by our contemporaries. :)

**隔离见证（Segregated Witness，简称SegWit）** 是比特币于2017年通过软分叉（BIP 141）引入的一项核心协议升级，旨在解决比特币网络的**交易延展性（Transaction Malleability）xx**问题，同时提升区块容量和脚本功能的灵活性。以下是其核心要点

---

假设观测数据 \(X = \{x_1, ..., x_n\}\) 服从正态分布 \(N(\mu, \sigma^2)\)，MLE的估计过程如下：
- **似然函数**：
  \[
  L(\mu, \sigma^2) = \prod_{i=1}^n \frac{1}{\sqrt{2\pi\sigma^2}} \exp\left(-\frac{(x_i - \mu)^2}{2\sigma^2}\right)
  \]
"#;

    // Would you like a deeper dive into any of these topics?

    // 2. **Add dependencies to your `Cargo.toml`**:
    //    ```toml
    //    [dependencies]
    //    reqwest = { version = "0.11", features = ["json"] }
    //    serde = { version = "1.0", features = ["derive"] }
    //    tokio = { version = "1.0", features = ["full"] }
    //    ```

    // | Header 1 | Header 2 | Header 3 |
    // |----------|----------|----------|
    // | Cell 1   | Cell 2   | Cell 5   |
    // | Cell 3   |          | Cell 6   |

    // $$\left( \sum_{k=1}^n a_k b_k \right)^2 \leq \left( \sum_{k=1}^n a_k^2 \right) \left( \sum_{k=1}^n b_k^2 \right)$$

    // $$
    // P_n(x) = f(a) + f'(a)(x-a) + \frac{f''(a)}{2!}(x-a)^2 + \cdots + \frac{f^{(n)}(a)}{n!}(x-a)^n
    // $$

    #[test]
    fn main() {
        let (ui_elems, link_urls) = parser::run(DOC);
        println!("{:#?}", ui_elems);
        println!("{:#?}", link_urls);
    }
}
