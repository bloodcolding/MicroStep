// 称号系统渲染：装备槽位（上限来自 /api/meta）+ 称号库（未解锁/已装备状态）。

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
        slot.title = "点击卸下";
        slot.dataset.unequipSlot = titleId;
      } else {
        slot.innerHTML = `<div><div class="emoji">＋</div><div class="name">空槽位</div></div>`;
      }
      slotHost.appendChild(slot);
    }
  }

  const host = document.getElementById("titleLibrary");
  if (!host) return;
  host.innerHTML = "";
  const titles = Object.values(state.titles || {});
  if (!titles.length) {
    host.innerHTML = '<p class="muted">创建里程碑后会生成对应的称号。</p>';
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
            ? '<button class="small-button" data-unequip-title="' + title.id + '">卸下</button>'
            : canEquip
              ? '<button class="small-button" data-equip-title="' + title.id + '">装备</button>'
              : unlocked
                ? '<span class="meta-pill">已装备</span>'
                : '<span class="meta-pill">未解锁</span>'
        }
      </div>
      <p>${escapeHtml(title.description)}</p>
      <p>加成：${dimensionName(title.target_dimension || "professional")} ${formatPercent(title.bonus_percent)}</p>
    `;
    host.appendChild(item);
  });
}
