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

/**
 * Minimal valid PDF file for testing.
 * Contains a single page with "Hello PDF" text.
 */
export function pdfFile(name: string): UploadableFile {
  const pdfContent = `%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R/Resources<</Font<</F1 4 0 R>>>>>>endobj
4 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000266 00000 n
trailer<</Size 5/Root 1 0 R>>
startxref
340
%%EOF`;
  return {
    name: name.endsWith(".pdf") ? name : `${name}.pdf`,
    mimeType: "application/pdf",
    buffer: Buffer.from(pdfContent, "utf-8"),
  };
}
