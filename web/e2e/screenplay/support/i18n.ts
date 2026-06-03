import zhCommon from "../../../messages/zh/common.json";
import zhErrors from "../../../messages/zh/errors.json";
import zhRoom from "../../../messages/zh/room.json";

type MessageBundle = Record<string, unknown>;

function resolveMessage(bundle: MessageBundle, key: string): string {
  const value = key.split(".").reduce<unknown>((current, segment) => {
    if (current && typeof current === "object" && segment in current) {
      return (current as MessageBundle)[segment];
    }
    return undefined;
  }, bundle);

  if (typeof value !== "string") {
    throw new Error(`Missing i18n key: ${key}`);
  }

  return value;
}

function formatMessage(template: string, values?: Record<string, string | number>): string {
  if (!values) {
    return template;
  }

  return template.replace(/\{(\w+)\}/g, (_match, name: string) => {
    const value = values[name];
    return value === undefined ? `{${name}}` : String(value);
  });
}

function translate(
  bundle: MessageBundle,
  key: string,
  values?: Record<string, string | number>,
): string {
  return formatMessage(resolveMessage(bundle, key), values);
}

export const tCommon = (key: string, values?: Record<string, string | number>) =>
  translate(zhCommon as MessageBundle, key, values);

export const tErrors = (key: string, values?: Record<string, string | number>) =>
  translate(zhErrors as MessageBundle, key, values);

export const tRoom = (key: string, values?: Record<string, string | number>) =>
  translate(zhRoom as MessageBundle, key, values);
