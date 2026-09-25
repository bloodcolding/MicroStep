// 称号系统渲染：装备槽位（上限来自 /api/meta）+ 称号库（未解锁/已装备状态）。

import { t } from "../i18n/index.js";
import { getState, getMeta } from "../state.js";
import { dimensionName } from "../dimensions.js";
import { escapeHtml, formatPercent } from "../utils.js";

export function renderTitles() {
  const state = getState();
  const maxSlots = Number(getMeta()?.max_equipped_titles) || 3;
  const slotHost = document.getElementById("equipmentSlots");
  if (slotHost) {
    slotHost.innerHTML = "";
    const equipped = state.equipped || [];
    for (let index = 0; index < maxSlots; index += 1) {
      const titleId = equipped[index];
      const title = titleId ? state.titles?.[titleId] : null;
      const slot = document.createElement("div");
      slot.className = `equipment-slot ${title ? "filled" : ""}`;
      if (title) {
        slot.innerHTML = `<div><div class="emoji">${title.emoji}</div><div class="name">${escapeHtml(title.name)}</div></div>`;
        slot.title = t("titles.unequipSlotTitle");
        slot.dataset.unequipSlot = titleId;
      } else {
        slot.innerHTML = `<div><div class="emoji">＋</div><div class="name">${t("titles.emptySlot")}</div></div>`;
      }
      slotHost.appendChild(slot);
    }
  }

  const host = document.getElementById("titleLibrary");
  if (!host) return;
  host.innerHTML = "";
  const titles = Object.values(state.titles || {});
  if (!titles.length) {
    host.innerHTML = `<p class="muted">${t("titles.empty")}</p>`;
    return;
  }
  titles.forEach((title) => {
    const unlocked = Boolean(title.unlocked);
    const isEquipped = state.equipped.includes(title.id);
    const canEquip = unlocked && !isEquipped && state.equipped.length < maxSlots;
    const item = document.createElement("div");
    item.className = `title-item ${unlocked ? "" : "locked"}`;
    item.innerHTML = `
      <div class="title-top">
        <span class="title-name">${title.emoji} ${escapeHtml(title.name)}</span>
        ${
          isEquipped
            ? `<button class="small-button" data-unequip-title="${title.id}">${t("titles.unequip")}</button>`
            : canEquip
              ? `<button class="small-button" data-equip-title="${title.id}">${t("titles.equip")}</button>`
              : unlocked
                ? `<span class="meta-pill">${t("titles.equipped")}</span>`
                : `<span class="meta-pill">${t("titles.locked")}</span>`
        }
      </div>
      <p>${escapeHtml(title.description)}</p>
      <p>${t("titles.bonus", { dimension: dimensionName(title.target_dimension || "professional"), percent: formatPercent(title.bonus_percent) })}</p>
    `;
    host.appendChild(item);
  });
}
