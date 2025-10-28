/**
 * Share Service
 *
 * This service handles sharing operations including:
 * - Generating share links
 * - Generating QR codes
 *
 * Note: This is a frontend-only service, no backend API calls are needed.
 */

// ============================================================================
// Share Functions
// ============================================================================

/**
 * Get share link for a room
 *
 * @param roomName - The name of the room
 * @returns The share URL
 */
export function getShareLink(roomName: string): string {
  if (typeof window === 'undefined') {
    return `https://elizabeth.app/room/${roomName}`
  }

  return `${window.location.origin}/room/${roomName}`
}

/**
 * Generate QR code image data URL for a room
 *
 * @param roomName - The name of the room
 * @returns Promise that resolves to a data URL of the QR code image
 */
export async function getQRCodeImage(roomName: string): Promise<string> {
  const shareLink = getShareLink(roomName)

  // TODO: Install qrcode library for better QR code generation
  // For now, use a QR code API service
  //
  // Alternative implementation:
  // import QRCode from 'qrcode'
  // return await QRCode.toDataURL(shareLink, { width: 300, margin: 2 })

  // Use external QR code API as temporary solution
  const qrApiUrl = `https://api.qrserver.com/v1/create-qr-code/?size=300x300&data=${encodeURIComponent(shareLink)}`
  return qrApiUrl
}

/**
 * Download QR code as an image file
 *
 * @param roomName - The name of the room
 * @param filename - Optional filename for the download
 */
export async function downloadQRCode(
  roomName: string,
  filename?: string
): Promise<void> {
  const qrImageUrl = await getQRCodeImage(roomName)

  // For API-generated QR codes, we can directly use the URL
  const link = document.createElement('a')
  link.href = qrImageUrl
  link.download = filename || `${roomName}-qr.png`
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
}

/**
 * Copy share link to clipboard
 *
 * @param roomName - The name of the room
 * @returns Promise that resolves when the link is copied
 */
export async function copyShareLink(roomName: string): Promise<void> {
  const shareLink = getShareLink(roomName)

  if (navigator.clipboard && navigator.clipboard.writeText) {
    await navigator.clipboard.writeText(shareLink)
  } else {
    // Fallback for older browsers
    const textArea = document.createElement('textarea')
    textArea.value = shareLink
    textArea.style.position = 'fixed'
    textArea.style.left = '-999999px'
    document.body.appendChild(textArea)
    textArea.focus()
    textArea.select()

    try {
      document.execCommand('copy')
    } finally {
      document.body.removeChild(textArea)
    }
  }
}

export default {
  getShareLink,
  getQRCodeImage,
  downloadQRCode,
  copyShareLink,
}
