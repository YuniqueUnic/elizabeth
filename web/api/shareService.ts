// Mock API service for sharing operations

// Mock: Get QR code image
export const getQRCodeImage = async (roomId: string): Promise<string> => {
  console.log("[API Mock] getQRCodeImage:", roomId)
  await new Promise((resolve) => setTimeout(resolve, 300))

  // Return a placeholder QR code image
  return "/placeholder.svg?height=200&width=200"
}

// Mock: Get share link
export const getShareLink = async (roomId: string): Promise<string> => {
  console.log("[API Mock] getShareLink:", roomId)
  await new Promise((resolve) => setTimeout(resolve, 300))

  return `https://elizabeth.app/room/${roomId}`
}
