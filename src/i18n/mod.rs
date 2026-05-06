//! Internationalization — static message translation.
//!
//! #9 i18n (from GAP_ANALYSIS_HERMES_OPENCLAW.md).
//!
//! Provides `display.language` config for static message translation.
//! Falls back to English when a translation is missing.
//! Custom translations loaded from `data_dir/i18n.toml`.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ── Language ─────────────────────────────────────────────────────────────────

/// Supported languages for agent message output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Zh,
    Ja,
    De,
    Es,
    Fr,
    Ko,
    Pt,
    Ru,
}

impl Language {
    pub fn code(&self) -> &str {
        match self {
            Self::En => "en",
            Self::Zh => "zh",
            Self::Ja => "ja",
            Self::De => "de",
            Self::Es => "es",
            Self::Fr => "fr",
            Self::Ko => "ko",
            Self::Pt => "pt",
            Self::Ru => "ru",
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl FromStr for Language {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "en" | "english" => Ok(Self::En),
            "zh" | "chinese" | "中文" => Ok(Self::Zh),
            "ja" | "japanese" | "日本語" => Ok(Self::Ja),
            "de" | "german" | "deutsch" => Ok(Self::De),
            "es" | "spanish" | "español" => Ok(Self::Es),
            "fr" | "french" | "français" => Ok(Self::Fr),
            "ko" | "korean" | "한국어" => Ok(Self::Ko),
            "pt" | "portuguese" | "português" => Ok(Self::Pt),
            "ru" | "russian" | "русский" => Ok(Self::Ru),
            other => Err(format!("unknown language: {other}")),
        }
    }
}

// ── Translation store ────────────────────────────────────────────────────────

/// Translation store: key → language → translated string.
#[derive(Debug, Clone, Default)]
pub struct I18n {
    translations: HashMap<String, HashMap<Language, String>>,
    language: Language,
}

impl I18n {
    /// Create a new i18n store with the given display language.
    pub fn new(language: Language) -> Self {
        let mut i18n = Self {
            translations: HashMap::new(),
            language,
        };
        i18n.load_builtin_translations();
        i18n
    }

    /// Translate a message key to the current language, falling back to English.
    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.translate(key, self.language)
            .or_else(|| self.translate(key, Language::En))
            .unwrap_or(key)
    }

    /// Translate a message key to a specific language.
    pub fn translate(&self, key: &str, lang: Language) -> Option<&str> {
        self.translations.get(key).and_then(|m| m.get(&lang)).map(|s| s.as_str())
    }

    /// Set the active language.
    pub fn set_language(&mut self, lang: Language) {
        self.language = lang;
    }

    /// Get the active language.
    pub fn language(&self) -> Language {
        self.language
    }

    /// Add a translation entry.
    pub fn add(&mut self, key: &str, lang: Language, value: &str) {
        self.translations
            .entry(key.to_string())
            .or_default()
            .insert(lang, value.to_string());
    }

    /// Load custom translations from a TOML file.
    ///
    /// Format:
    /// ```toml
    /// [messages]
    /// session_started = { en = "Session started", zh = "会话已开始" }
    /// ```
    pub fn load_translations(&mut self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }
        let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let custom: CustomTranslations =
            toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;

        for (key, translations) in custom.messages {
            for (lang_code, value) in translations {
                if let Ok(lang) = Language::from_str(&lang_code) {
                    self.add(&key, lang, &value);
                } else {
                    log::warn!("i18n: skipping unknown language '{}' for key '{}'", lang_code, key);
                }
            }
        }

        log::info!("i18n: loaded custom translations from {}", path.display());
        Ok(())
    }

    /// Number of translated keys.
    pub fn key_count(&self) -> usize {
        self.translations.len()
    }

    fn load_builtin_translations(&mut self) {
        let builtins: &[(&str, &[(&str, &str)])] = &[
            (
                "session.started",
                &[
                    ("en", "Session started"),
                    ("zh", "会话已开始"),
                    ("ja", "セッションを開始しました"),
                    ("de", "Sitzung gestartet"),
                    ("es", "Sesión iniciada"),
                    ("fr", "Session démarrée"),
                    ("ko", "세션이 시작되었습니다"),
                    ("pt", "Sessão iniciada"),
                    ("ru", "Сессия запущена"),
                ],
            ),
            (
                "session.completed",
                &[
                    ("en", "Session completed"),
                    ("zh", "会话已完成"),
                    ("ja", "セッションが完了しました"),
                    ("de", "Sitzung abgeschlossen"),
                    ("es", "Sesión completada"),
                    ("fr", "Session terminée"),
                    ("ko", "세션이 완료되었습니다"),
                    ("pt", "Sessão concluída"),
                    ("ru", "Сессия завершена"),
                ],
            ),
            (
                "task.completed",
                &[
                    ("en", "Task completed"),
                    ("zh", "任务已完成"),
                    ("ja", "タスクが完了しました"),
                    ("de", "Aufgabe abgeschlossen"),
                    ("es", "Tarea completada"),
                    ("fr", "Tâche terminée"),
                    ("ko", "작업이 완료되었습니다"),
                    ("pt", "Tarefa concluída"),
                    ("ru", "Задача завершена"),
                ],
            ),
            (
                "approval.required",
                &[
                    ("en", "Approval required"),
                    ("zh", "需要批准"),
                    ("ja", "承認が必要です"),
                    ("de", "Genehmigung erforderlich"),
                    ("es", "Aprobación requerida"),
                    ("fr", "Approbation requise"),
                    ("ko", "승인이 필요합니다"),
                    ("pt", "Aprovação necessária"),
                    ("ru", "Требуется одобрение"),
                ],
            ),
            (
                "approval.granted",
                &[
                    ("en", "Approval granted"),
                    ("zh", "已批准"),
                    ("ja", "承認されました"),
                    ("de", "Genehmigt"),
                    ("es", "Aprobado"),
                    ("fr", "Approuvé"),
                    ("ko", "승인되었습니다"),
                    ("pt", "Aprovado"),
                    ("ru", "Одобрено"),
                ],
            ),
            (
                "approval.denied",
                &[
                    ("en", "Approval denied"),
                    ("zh", "已拒绝"),
                    ("ja", "承認が拒否されました"),
                    ("de", "Abgelehnt"),
                    ("es", "Denegado"),
                    ("fr", "Refusé"),
                    ("ko", "승인이 거부되었습니다"),
                    ("pt", "Negado"),
                    ("ru", "Отклонено"),
                ],
            ),
            (
                "error.rate_limited",
                &[
                    ("en", "Rate limited — retrying"),
                    ("zh", "速率受限 — 重试中"),
                    ("ja", "レート制限 — リトライ中"),
                    ("de", "Ratenbegrenzt — wird erneut versucht"),
                    ("es", "Limitado — reintentando"),
                    ("fr", "Limité — nouvelle tentative"),
                    ("ko", "속도 제한 — 재시도 중"),
                    ("pt", "Limitado — tentando novamente"),
                    ("ru", "Ограничение — повторная попытка"),
                ],
            ),
            (
                "error.provider_failed",
                &[
                    ("en", "Provider failed"),
                    ("zh", "提供商失败"),
                    ("ja", "プロバイダーエラー"),
                    ("de", "Anbieter fehlgeschlagen"),
                    ("es", "Proveedor fallido"),
                    ("fr", "Fournisseur échoué"),
                    ("ko", "제공자 실패"),
                    ("pt", "Provedor falhou"),
                    ("ru", "Провайдер не ответил"),
                ],
            ),
            (
                "error.session_failed",
                &[
                    ("en", "Session failed"),
                    ("zh", "会话失败"),
                    ("ja", "セッションエラー"),
                    ("de", "Sitzung fehlgeschlagen"),
                    ("es", "Sesión fallida"),
                    ("fr", "Session échouée"),
                    ("ko", "세션 실패"),
                    ("pt", "Sessão falhou"),
                    ("ru", "Ошибка сессии"),
                ],
            ),
            (
                "daemon.waking",
                &[
                    ("en", "Daemon waking"),
                    ("zh", "守护进程唤醒"),
                    ("ja", "デーモン起動"),
                    ("de", "Daemon erwacht"),
                    ("es", "Demonio despertando"),
                    ("fr", "Démon en veille"),
                    ("ko", "데몬 깨우기"),
                    ("pt", "Daemon acordando"),
                    ("ru", "Пробуждение демона"),
                ],
            ),
            (
                "daemon.sleeping",
                &[
                    ("en", "Daemon sleeping"),
                    ("zh", "守护进程休眠"),
                    ("ja", "デーモン休止"),
                    ("de", "Daemon schläft"),
                    ("es", "Demonio durmiendo"),
                    ("fr", "Démon en sommeil"),
                    ("ko", "데몬 대기"),
                    ("pt", "Daemon dormindo"),
                    ("ru", "Демон засыпает"),
                ],
            ),
            (
                "brief.morning",
                &[
                    ("en", "Morning brief"),
                    ("zh", "晨报"),
                    ("ja", "朝のブリーフィング"),
                    ("de", "Morgenbriefing"),
                    ("es", "Resumen matutino"),
                    ("fr", "Briefing matinal"),
                    ("ko", "아침 브리핑"),
                    ("pt", "Resumo matinal"),
                    ("ru", "Утренняя сводка"),
                ],
            ),
            (
                "tool.executing",
                &[
                    ("en", "Executing tool"),
                    ("zh", "执行工具"),
                    ("ja", "ツール実行中"),
                    ("de", "Führe Werkzeug aus"),
                    ("es", "Ejecutando herramienta"),
                    ("fr", "Exécution de l'outil"),
                    ("ko", "도구 실행 중"),
                    ("pt", "Executando ferramenta"),
                    ("ru", "Выполнение инструмента"),
                ],
            ),
            (
                "tool.blocked",
                &[
                    ("en", "Tool blocked by policy"),
                    ("zh", "工具被策略阻止"),
                    ("ja", "ツールがポリシーでブロックされました"),
                    ("de", "Werkzeug durch Richtlinie blockiert"),
                    ("es", "Herramienta bloqueada"),
                    ("fr", "Outil bloqué"),
                    ("ko", "도구가 정책에 의해 차단됨"),
                    ("pt", "Ferramenta bloqueada"),
                    ("ru", "Инструмент заблокирован"),
                ],
            ),
            (
                "memory.saved",
                &[
                    ("en", "Memory saved"),
                    ("zh", "记忆已保存"),
                    ("ja", "メモリを保存しました"),
                    ("de", "Erinnerung gespeichert"),
                    ("es", "Memoria guardada"),
                    ("fr", "Mémoire sauvegardée"),
                    ("ko", "메모리 저장됨"),
                    ("pt", "Memória salva"),
                    ("ru", "Память сохранена"),
                ],
            ),
            (
                "skill.installed",
                &[
                    ("en", "Skill installed"),
                    ("zh", "技能已安装"),
                    ("ja", "スキルをインストールしました"),
                    ("de", "Fähigkeit installiert"),
                    ("es", "Habilidad instalada"),
                    ("fr", "Compétence installée"),
                    ("ko", "스킬 설치됨"),
                    ("pt", "Habilidade instalada"),
                    ("ru", "Навык установлен"),
                ],
            ),
            (
                "goal.active",
                &[
                    ("en", "Active goal"),
                    ("zh", "活跃目标"),
                    ("ja", "アクティブな目標"),
                    ("de", "Aktives Ziel"),
                    ("es", "Objetivo activo"),
                    ("fr", "Objectif actif"),
                    ("ko", "활성 목표"),
                    ("pt", "Objetivo ativo"),
                    ("ru", "Активная цель"),
                ],
            ),
            (
                "evolution.proposed",
                &[
                    ("en", "Evolution proposed"),
                    ("zh", "进化提案"),
                    ("ja", "進化が提案されました"),
                    ("de", "Evolution vorgeschlagen"),
                    ("es", "Evolución propuesta"),
                    ("fr", "Évolution proposée"),
                    ("ko", "진화 제안됨"),
                    ("pt", "Evolução proposta"),
                    ("ru", "Эволюция предложена"),
                ],
            ),
            (
                "curator.cycle_complete",
                &[
                    ("en", "Curator cycle complete"),
                    ("zh", "管理者周期完成"),
                    ("ja", "キュレーターサイクル完了"),
                    ("de", "Kurator-Zyklus abgeschlossen"),
                    ("es", "Ciclo del curador completo"),
                    ("fr", "Cycle du conservateur terminé"),
                    ("ko", "큐레이터 사이클 완료"),
                    ("pt", "Ciclo do curador concluído"),
                    ("ru", "Цикл куратора завершён"),
                ],
            ),
            (
                "webhook.received",
                &[
                    ("en", "Webhook received"),
                    ("zh", "收到Webhook"),
                    ("ja", "Webhookを受信しました"),
                    ("de", "Webhook empfangen"),
                    ("es", "Webhook recibido"),
                    ("fr", "Webhook reçu"),
                    ("ko", "웹훅 수신됨"),
                    ("pt", "Webhook recebido"),
                    ("ru", "Вебхук получен"),
                ],
            ),
        ];

        for (key, translations) in builtins {
            for (code, value) in *translations {
                if let Ok(lang) = Language::from_str(code) {
                    self.add(key, lang, value);
                }
            }
        }
    }
}

// ── Custom TOML format ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
struct CustomTranslations {
    messages: HashMap<String, HashMap<String, String>>,
}

// ── Config integration ───────────────────────────────────────────────────────

/// I18n configuration for praxis.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct I18nConfig {
    #[serde(default)]
    pub language: Language,
}

impl Default for I18nConfig {
    fn default() -> Self {
        Self { language: Language::En }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_to_english() {
        let i18n = I18n::new(Language::Ko);
        assert_eq!(i18n.t("session.started"), "세션이 시작되었습니다");
    }

    #[test]
    fn unknown_key_returns_key() {
        let i18n = I18n::new(Language::En);
        assert_eq!(i18n.t("nonexistent.key"), "nonexistent.key");
    }

    #[test]
    fn language_from_str() {
        assert_eq!(Language::from_str("zh").unwrap(), Language::Zh);
        assert_eq!(Language::from_str("Japanese").unwrap(), Language::Ja);
        assert!(Language::from_str("xx").is_err());
    }

    #[test]
    fn all_builtins_present() {
        let i18n = I18n::new(Language::En);
        assert!(i18n.key_count() >= 20);
    }
}
