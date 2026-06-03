import { API_BASE_URL } from "@/lib/config";
import type { FileItem } from "@/lib/types";

const ABSOLUTE_URL_REGEX = /^https?:\/\//i;
const IMAGE_FILE_REGEX = /\.(jpg|jpeg|png|gif|webp|svg)$/i;

export function isAbsoluteUrl(value?: string | null): value is string {
  return typeof value === "string" && ABSOLUTE_URL_REGEX.test(value);
}

export function buildContentPreviewPath(contentId: string): string {
  return `/contents/${contentId}`;
}

export function buildContentAssetPath(contentId: string): string {
  return `${API_BASE_URL}/contents/${contentId}`;
}

export function appendToken(url: string, token?: string): string {
  if (!token) {
    return url;
  }

  const urlObject = new URL(url, "http://elizabeth.local");
  urlObject.searchParams.set("token", token);

  if (isAbsoluteUrl(url)) {
    return urlObject.toString();
  }

  return `${urlObject.pathname}${urlObject.search}${urlObject.hash}`;
}

export function toAbsoluteUrl(url: string, origin: string): string {
  if (isAbsoluteUrl(url)) {
    return url;
  }

  return new URL(url, origin).toString();
}

export function isImageFile(file: Pick<FileItem, "name" | "type">): boolean {
  return file.type === "image" || IMAGE_FILE_REGEX.test(file.name);
}

export function resolveFilePreviewHref(
  file: Pick<FileItem, "id" | "type" | "url">,
): string | undefined {
  if (file.type === "link") {
    return file.url;
  }

  return file.url ?? buildContentPreviewPath(file.id);
}

export function resolveFileAssetPath(
  file: Pick<FileItem, "id" | "type" | "assetUrl" | "url">,
): string | undefined {
  if (file.type === "link") {
    return file.assetUrl ?? file.url;
  }

  return file.assetUrl ?? buildContentAssetPath(file.id);
}

export function buildMarkdownReference(
  file: Pick<FileItem, "name" | "type">,
  href: string,
): string {
  if (isImageFile(file)) {
    return `![](${href})`;
  }

  return `[${file.name}](${href})`;
}
