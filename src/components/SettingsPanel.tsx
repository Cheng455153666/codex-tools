import { useEffect } from "react";
import { EditorMultiSelect } from "./EditorMultiSelect";
import { ThemeSwitch } from "./ThemeSwitch";
import { SwitchField } from "./SwitchField";
import type {
  AppSettings,
  InstalledEditorApp,
  ThemeMode,
  UpdateSettingsOptions,
} from "../types/app";

type SettingsPanelProps = {
  open: boolean;
  onClose: () => void;
  themeMode: ThemeMode;
  onToggleTheme: () => void;
  settings: AppSettings;
  installedEditorApps: InstalledEditorApp[];
  savingSettings: boolean;
  onUpdateSettings: (patch: Partial<AppSettings>, options?: UpdateSettingsOptions) => void;
};

export function SettingsPanel({
  open,
  onClose,
  themeMode,
  onToggleTheme,
  settings,
  installedEditorApps,
  savingSettings,
  onUpdateSettings,
}: SettingsPanelProps) {
  useEffect(() => {
    if (!open) {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [onClose, open]);

  if (!open) {
    return null;
  }

  return (
    <div className="settingsOverlay" onClick={onClose}>
      <section
        className="settingsDialog"
        role="dialog"
        aria-modal="true"
        aria-label="应用设置"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="settingsHeader">
          <div>
            <h2>设置</h2>
            <p>可配置开机启动、状态栏显示模式和主题。</p>
          </div>
          <button className="iconButton ghost" onClick={onClose} aria-label="关闭设置" title="关闭">
            <svg className="iconGlyph" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
              <path d="m6 6 12 12" />
              <path d="M18 6 6 18" />
            </svg>
          </button>
        </div>

        <SwitchField
          checked={settings.launchAtStartup}
          onChange={(checked) => onUpdateSettings({ launchAtStartup: checked })}
          label="开机启动"
          description="启用后会在系统登录时自动启动 Codex Tools。"
          checkedText="开启"
          uncheckedText="关闭"
          disabled={savingSettings}
        />

        <SwitchField
          checked={settings.launchCodexAfterSwitch}
          onChange={(checked) => onUpdateSettings({ launchCodexAfterSwitch: checked })}
          label="切换后启动 Codex"
          description="默认开启。关闭时仅切换账号，不自动拉起 Codex。"
          checkedText="启动"
          uncheckedText="仅切换"
          disabled={savingSettings}
        />

        <SwitchField
          checked={settings.syncOpencodeOpenaiAuth}
          onChange={(checked) => onUpdateSettings({ syncOpencodeOpenaiAuth: checked })}
          label="同步 Opencode OpenAI"
          description="切换账号时自动探测 opencode 认证文件，并同步 refresh/access。"
          checkedText="同步"
          uncheckedText="不同步"
          disabled={savingSettings}
        />

        <SwitchField
          checked={settings.restartEditorsOnSwitch}
          onChange={(checked) => {
            if (checked && settings.restartEditorTargets.length === 0 && installedEditorApps.length > 0) {
              onUpdateSettings({
                restartEditorsOnSwitch: true,
                restartEditorTargets: [installedEditorApps[0].id],
              });
              return;
            }
            onUpdateSettings({ restartEditorsOnSwitch: checked });
          }}
          label="切换时重启编辑器(兼容codex 编辑器插件)"
          description="默认关闭。开启后切换账号会强制关闭并重启你选中的编辑器。"
          checkedText="重启"
          uncheckedText="不重启"
          disabled={savingSettings}
        />

        <div className="settingRow">
          <div className="settingMeta">
            <strong>重启目标编辑器（单选）</strong>
            <p>后台自动检测已安装的 VSCode/VSCode Insiders/Cursor/Antigravity/Kiro/Trae/Qoder。</p>
          </div>
          <EditorMultiSelect
            options={installedEditorApps}
            value={settings.restartEditorTargets[0] ?? null}
            disabled={installedEditorApps.length === 0}
            onChange={(selected) =>
              onUpdateSettings(
                { restartEditorTargets: [selected] },
                { silent: true, keepInteractive: true },
              )
            }
          />
        </div>
        {installedEditorApps.length === 0 && (
          <p className="hint">当前未检测到支持重启的编辑器。</p>
        )}

        <div className="settingRow">
          <div className="settingMeta">
            <strong>状态栏展示</strong>
            <p>控制状态栏菜单中显示“已用”还是“剩余”。</p>
          </div>
          <div className="modeGroup" role="radiogroup" aria-label="状态栏展示模式">
            <button
              className={settings.trayUsageDisplayMode === "remaining" ? "primary" : "ghost"}
              disabled={savingSettings}
              onClick={() => onUpdateSettings({ trayUsageDisplayMode: "remaining" })}
              aria-pressed={settings.trayUsageDisplayMode === "remaining"}
            >
              剩余
            </button>
            <button
              className={settings.trayUsageDisplayMode === "used" ? "primary" : "ghost"}
              disabled={savingSettings}
              onClick={() => onUpdateSettings({ trayUsageDisplayMode: "used" })}
              aria-pressed={settings.trayUsageDisplayMode === "used"}
            >
              已用
            </button>
          </div>
        </div>

        <div className="settingRow">
          <div className="settingMeta">
            <strong>主题</strong>
            <p>使用开关切换浅色和深色主题。</p>
          </div>
          <ThemeSwitch themeMode={themeMode} onToggle={onToggleTheme} />
        </div>
      </section>
    </div>
  );
}
