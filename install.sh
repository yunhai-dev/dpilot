#!/bin/sh
# 从 GitHub Release 安装 dpilot；无需 Rust 或 root 权限。
set -eu

main() {
    for tool in curl tar mktemp install uname; do
        command -v "$tool" >/dev/null 2>&1 || { printf '缺少依赖：%s\n' "$tool" >&2; exit 1; }
    done
    case "$(uname -s)" in
        Linux) os=unknown-linux-gnu ;;
        Darwin) os=apple-darwin ;;
        *) printf '%s\n' '仅支持 Linux/macOS；Windows 请从 Releases 下载 ZIP。' >&2; exit 1 ;;
    esac
    case "$(uname -m)" in
        x86_64|amd64) arch=x86_64 ;;
        arm64|aarch64) arch=aarch64 ;;
        *) printf '%s\n' '当前 CPU 架构没有预编译产物。' >&2; exit 1 ;;
    esac
    if command -v sha256sum >/dev/null 2>&1; then
        checksum=sha256sum
    elif command -v shasum >/dev/null 2>&1; then
        checksum=shasum
    else
        printf '%s\n' '缺少 SHA256 校验工具：需要 sha256sum 或 shasum。' >&2
        exit 1
    fi

    repo=https://github.com/yunhai-dev/dpilot
    # 先固定最新正式版本，避免下载期间新版本发布导致校验文件与压缩包不一致。
    release_url=$(curl --proto '=https' --tlsv1.2 -fsSL -o /dev/null -w '%{url_effective}' "$repo/releases/latest")
    version=${release_url##*/}
    case "$version" in
        v[0-9]*) ;;
        *) printf '%s\n' '无法确定最新 Release 版本。' >&2; exit 1 ;;
    esac
    archive="dpilot-${version}-${arch}-${os}.tar.gz"
    base="$repo/releases/download/$version"
    install_dir=${DPILOT_INSTALL_DIR:-"$HOME/.local/bin"}
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    trap 'exit 1' HUP INT TERM

    printf '下载 %s\n' "$archive"
    curl --proto '=https' --tlsv1.2 -fsSL "$base/SHA256SUMS" -o "$tmp/SHA256SUMS"
    curl --proto '=https' --tlsv1.2 -fsSL "$base/$archive" -o "$tmp/$archive"
    (
        cd "$tmp"
        matches=0
        while read -r digest filename; do
            if [ "$filename" = "$archive" ]; then
                printf '%s  %s\n' "$digest" "$filename" > selected.sha256
                matches=$((matches + 1))
            fi
        done < SHA256SUMS
        [ "$matches" -eq 1 ] || { printf '%s\n' '未找到唯一的产物校验值。' >&2; exit 1; }
        if [ "$checksum" = sha256sum ]; then
            sha256sum -c selected.sha256
        else
            shasum -a 256 -c selected.sha256
        fi
        tar -xzf "$archive" dpilot
        # 先验证二进制兼容性，失败时不替换原有安装。
        ./dpilot --version
    )
    mkdir -p "$install_dir"
    install -m 755 "$tmp/dpilot" "$install_dir/dpilot"
    printf '安装完成：%s/dpilot\n' "$install_dir"
    case ":$PATH:" in
        *":$install_dir:"*) ;;
        *) printf '请将 %s 加入 PATH，或使用上述完整路径运行。\n' "$install_dir" ;;
    esac
}

main "$@"
