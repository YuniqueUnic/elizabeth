declare module "react-file-icon" {
  import { FC } from "react";

  export interface FileIconProps {
    extension?: string;
    color?: string;
    fold?: boolean;
    foldColor?: string;
    glyphColor?: string;
    gradientColor?: string;
    gradientOpacity?: number;
    labelColor?: string;
    labelTextColor?: string;
    labelUppercase?: boolean;
    radius?: number;
    type?:
      | "3d"
      | "acrobat"
      | "android"
      | "audio"
      | "binary"
      | "code"
      | "compressed"
      | "document"
      | "drive"
      | "font"
      | "image"
      | "presentation"
      | "settings"
      | "spreadsheet"
      | "vector"
      | "video";
  }

  export const FileIcon: FC<FileIconProps>;

  export const defaultStyles: Record<string, Partial<FileIconProps>>;
}
