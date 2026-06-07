// 把 source.svg 渲染成 1024x1024 PNG，供 `tauri icon` 生成各平台图标
const sharp = require("sharp");

const input = process.argv[2] || "src-tauri/icons/source.svg";

sharp(input)
  .resize(1024, 1024)
  .png()
  .toFile("src-tauri/icons/source.png")
  .then(() => console.log("OK -> src-tauri/icons/source.png"))
  .catch((e) => {
    console.error(e);
    process.exit(1);
  });
