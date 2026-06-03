export interface UploadableFile {
  name: string;
  mimeType: string;
  buffer: Buffer;
}

export function uniqueRoomName(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.floor(Math.random() * 1_000_000)}`;
}

export function textFile(
  name: string,
  content: string,
  mimeType = "text/plain",
): UploadableFile {
  return {
    name,
    mimeType,
    buffer: Buffer.from(content, "utf-8"),
  };
}

export function markdownFile(name: string, content: string): UploadableFile {
  return textFile(name, content, "text/markdown");
}

export function jsonFile(name: string, data: unknown): UploadableFile {
  return {
    name,
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(data, null, 2), "utf-8"),
  };
}

export function binaryFile(
  name: string,
  mimeType: string,
  bytes: Buffer,
): UploadableFile {
  return {
    name,
    mimeType,
    buffer: bytes,
  };
}

export function pngFile(name: string): UploadableFile {
  return binaryFile(
    name,
    "image/png",
    Buffer.from(
      "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO7ZxE0AAAAASUVORK5CYII=",
      "base64",
    ),
  );
}
