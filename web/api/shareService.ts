/**
 * Share Service
 *
 * 房间分享只需要两件事：分享链接本身，以及它的二维码。
 * 链接就是房间自身的 URL —— 访问者打开后由房间入口完成鉴权（密码 / 身份码），
 * 因此这里不做任何权限判定或可用性探测；是否展示分享入口由调用方按 `room.share` 能力决定。
 */

import QRCode from "qrcode";

/** 房间分享链接。 */
export function getShareLink(roomName: string): string {
  return `${window.location.origin}/${roomName}`;
}

/** 生成分享链接的二维码（data URL），配色随主题反转以保证可扫描。 */
export async function getQRCodeImage(
  roomName: string,
  theme: "light" | "dark",
): Promise<string> {
  return await QRCode.toDataURL(getShareLink(roomName), {
    width: 300,
    margin: 2,
    errorCorrectionLevel: "M",
    color: theme === "dark"
      ? { dark: "#FFFFFF", light: "#1e293b" }
      : { dark: "#1e293b", light: "#FFFFFF" },
  });
}
