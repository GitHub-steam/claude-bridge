#!/usr/bin/env bash
#
# 一键发版（Windows）：签名打包 → 生成 latest.json → 建 GitHub Release → 上传安装包+清单
#
# 用法：
#   1) 先把版本号升好（4 处）：package.json / src-tauri/Cargo.toml /
#      src-tauri/tauri.conf.json / src/App.tsx 里的 APP_VERSION
#   2) 设好签名私钥路径（绝不要把私钥放进仓库）：
#        export CB_SIGNING_KEY=/path/to/cb_updater.key
#        export CB_SIGNING_PASSWORD=""        # 若私钥有密码则填上
#   3) 运行：  bash release.sh
#
set -euo pipefail
cd "$(dirname "$0")"

REPO="GitHub-steam/claude-bridge"

: "${CB_SIGNING_KEY:?请先 export CB_SIGNING_KEY=私钥文件路径}"
[ -f "$CB_SIGNING_KEY" ] || { echo "找不到私钥文件：$CB_SIGNING_KEY"; exit 1; }

VERSION=$(node -p "require('./package.json').version")
TAG="v$VERSION"
NSIS="src-tauri/target/release/bundle/nsis"
EXE="ClaudeBridge_${VERSION}_x64-setup.exe"

echo ">> 打包 ClaudeBridge $TAG（签名）..."
export TAURI_SIGNING_PRIVATE_KEY="$(cat "$CB_SIGNING_KEY")"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${CB_SIGNING_PASSWORD:-}"
npm run tauri build -- --bundles nsis

[ -f "$NSIS/$EXE.sig" ] || { echo "未找到签名产物，确认 tauri.conf 里 createUpdaterArtifacts=true"; exit 1; }

echo ">> 生成 latest.json ..."
SIG=$(cat "$NSIS/$EXE.sig")
PUB=$(date -u +%Y-%m-%dT%H:%M:%SZ)
cat > "$NSIS/latest.json" <<EOF
{
  "version": "$VERSION",
  "notes": "ClaudeBridge $TAG",
  "pub_date": "$PUB",
  "platforms": {
    "windows-x86_64": {
      "signature": "$SIG",
      "url": "https://github.com/$REPO/releases/download/$TAG/$EXE"
    }
  }
}
EOF

echo ">> 建 Release 并上传 ..."
gh release create "$TAG" --title "ClaudeBridge $TAG" --notes "ClaudeBridge $TAG" \
  "$NSIS/$EXE" "$NSIS/latest.json"

echo ">> 完成：https://github.com/$REPO/releases/tag/$TAG"
echo "   （可到网页编辑该 Release 的说明文字）"
