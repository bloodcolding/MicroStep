// 运行时 i18n（add-frontend-i18n）：字典查找、插值、简单复数、缺失回退、
// 语言持久化与静态文案批量应用。只依赖字典模块，不 import 业务模块。

import { dictionary as zhCN } from "./locales/zh-CN.js";
import { dictionary as en } from "./locales/en.js";

const SUPPORTED_LOCALES = ["zh-CN", "en"];
const FALLBACK_LOCALE = "zh-CN";
const STORAGE_KEY = "microstep.locale";

const dictionaries = { "zh-CN": zhCN, en };
const listeners = [];
const warnedKeys = new Set();
let currentLocale = FALLBACK_LOCALE;

function isSupportedLocale(locale) {
  return SUPPORTED_LOCALES.includes(locale);
}

// 语言解析链：localStorage 合法值 → navigator.language（zh* → zh-CN，否则 en）→ zh-CN 兜底。
export function resolveInitialLocale() {
  let stored = null;
  try {
    stored = window.localStorage.getItem(STORAGE_KEY);
  } catch {
    stored = null;
  }
  if (isSupportedLocale(stored)) return stored;
  const nav = (Array.isArray(navigator.languages) && navigator.languages[0]) || navigator.language || "";
  if (String(nav).trim().toLowerCase().startsWith("zh")) return "zh-CN";
  if (String(nav).trim()) return "en";
  return FALLBACK_LOCALE;
}

export function initI18n() {
  currentLocale = resolveInitialLocale();
  applyStaticTranslations();
  return currentLocale;
}

export function getLocale() {
  return currentLocale;
}

// 数字 / 日期格式化 locale（R6：zh-CN → zh-CN，en → en-US）。
export function currentNumberLocale() {
  return currentLocale === "en" ? "en-US" : "zh-CN";
}

function warnMissing(key) {
  if (warnedKeys.has(key)) return;
  warnedKeys.add(key);
  console.warn("AGENT-I18N:", key);
}

function lookup(locale, key) {
  const dict = dictionaries[locale];
  if (!dict || !Object.prototype.hasOwnProperty.call(dict, key)) return undefined;
  return dict[key];
}

// 无告警字典读取：供 dimensionName 这类自带三级兜底的调用方使用。
export function dictValue(key) {
  return lookup(currentLocale, key) ?? lookup(FALLBACK_LOCALE, key);
}

export function hasTranslation(key) {
  return dictValue(key) !== undefined;
}

function interpolate(template, params) {
  if (!params) return template;
  return String(template).replace(/\{\{(\w+)\}\}/g, (placeholder, name) =>
    params[name] === undefined || params[name] === null ? placeholder : String(params[name])
  );
}

// 翻译函数：插值 + 简单复数（count → Intl.PluralRules 选 .one/.other，无 count 用 .other）
// + 当前语言缺失时回退 zh-CN 值并 console.warn（每 key 一次），最终兜底返回 key 原文。
export function t(key, params = {}) {
  let resolved;
  if (params && typeof params.count === "number" && Number.isFinite(params.count)) {
    const category = new Intl.PluralRules(currentNumberLocale()).select(params.count);
    resolved = lookup(currentLocale, `${key}.${category}`);
    if (resolved === undefined && category !== "other") {
      resolved = lookup(currentLocale, `${key}.other`);
    }
    if (resolved === undefined) {
      resolved = lookup(FALLBACK_LOCALE, `${key}.${category}`);
      if (resolved === undefined && category !== "other") {
        resolved = lookup(FALLBACK_LOCALE, `${key}.other`);
      }
    }
  }
  if (resolved === undefined) resolved = lookup(currentLocale, key);
  if (resolved === undefined) resolved = lookup(currentLocale, `${key}.other`);
  if (resolved === undefined) {
    const fallback = lookup(FALLBACK_LOCALE, key) ?? lookup(FALLBACK_LOCALE, `${key}.other`);
    warnMissing(key);
    if (fallback === undefined) return key;
    resolved = fallback;
  }
  return interpolate(resolved, params);
}

export function setLocale(locale) {
  if (!isSupportedLocale(locale) || locale === currentLocale) return currentLocale;
  currentLocale = locale;
  try {
    window.localStorage.setItem(STORAGE_KEY, locale);
  } catch {
    // 本地存储不可用时仅会话内生效。
  }
  applyStaticTranslations();
  listeners.forEach((listener) => {
    try {
      listener(locale);
    } catch (error) {
      console.error("AGENT-I18N: listener failed", error);
    }
  });
  return currentLocale;
}

// 顶栏切换按钮：zh-CN ↔ en 互切。
export function toggleLocale() {
  return setLocale(currentLocale === "zh-CN" ? "en" : "zh-CN");
}

export function onLocaleChanged(listener) {
  listeners.push(listener);
  return () => {
    const index = listeners.indexOf(listener);
    if (index >= 0) listeners.splice(index, 1);
  };
}

// 启动与切换时批量应用 index.html 的声明式标记；<title> 与 <html lang> 一并同步。
export function applyStaticTranslations() {
  document.documentElement.lang = currentLocale;
  document.querySelectorAll("[data-i18n]").forEach((element) => {
    element.textContent = t(element.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-placeholder]").forEach((element) => {
    element.placeholder = t(element.dataset.i18nPlaceholder);
  });
  document.querySelectorAll("[data-i18n-aria-label]").forEach((element) => {
    element.setAttribute("aria-label", t(element.dataset.i18nAriaLabel));
  });
}
