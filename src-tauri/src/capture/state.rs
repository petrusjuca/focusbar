//! Máquina de estados de presença (pura, sem I/O — testável em qualquer SO).
//! Decide o que cada "tick" representa a partir de sinais simples: tempo ocioso,
//! áudio tocando, tela bloqueada e pausa manual. É o que separa "viu um vídeo
//! de 40min" (passivo) de "trabalhou 40min" (ativo) e de "saiu do PC" (ausente).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickState {
    Active,        // mexendo (idle < 60s)
    Passive,       // parado mas com áudio (assistindo/ouvindo)
    Idle,          // parado curto, sem áudio (1–5min)
    Away,          // ausente (>=5min sem áudio, ou tela bloqueada)
    AwayUncertain, // muito tempo parado mas com áudio tocando (>=15min) — incerto
    Paused,        // monitoramento desligado manualmente
}

impl TickState {
    /// Rótulo curto pra UI/eventos.
    pub fn as_str(self) -> &'static str {
        match self {
            TickState::Active => "active",
            TickState::Passive => "passive",
            TickState::Idle => "idle",
            TickState::Away => "away",
            TickState::AwayUncertain => "away_uncertain",
            TickState::Paused => "paused",
        }
    }
    /// É um estado de AUSÊNCIA que vira marcador na timeline?
    pub fn marker_kind(self) -> Option<&'static str> {
        match self {
            TickState::Away => Some("away"),
            TickState::AwayUncertain => Some("away_uncertain"),
            _ => None,
        }
    }
}

const ACTIVE_MAX: i64 = 60; // < 60s sem input = ativo
const IDLE_SHORT_MAX: i64 = 300; // < 5min = ocioso curto
const AWAY_MIN: i64 = 900; // >= 15min = ausência longa (mesmo com áudio = incerto)

/// Resolve o estado do tick na ORDEM de prioridade (a primeira que casa vence).
pub fn resolve_state(idle_secs: i64, audio_active: bool, locked: bool, paused: bool) -> TickState {
    if paused {
        return TickState::Paused;
    }
    if locked {
        return TickState::Away;
    }
    if idle_secs < ACTIVE_MAX {
        return TickState::Active;
    }
    if idle_secs < IDLE_SHORT_MAX {
        // parado de 1 a 5min: com áudio = consumindo; sem = ocioso curto.
        return if audio_active {
            TickState::Passive
        } else {
            TickState::Idle
        };
    }
    if idle_secs < AWAY_MIN {
        // 5 a 15min parado: ausente (mesmo com áudio — pode ser autoplay).
        return TickState::Away;
    }
    // >= 15min parado: ausente. Com áudio tocando, marca como "incerto".
    if audio_active {
        TickState::AwayUncertain
    } else {
        TickState::Away
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paused_beats_tudo() {
        assert_eq!(resolve_state(0, true, true, true), TickState::Paused);
    }
    #[test]
    fn locked_beats_active() {
        assert_eq!(resolve_state(0, false, true, false), TickState::Away);
    }
    #[test]
    fn active_quando_mexendo() {
        assert_eq!(resolve_state(10, false, false, false), TickState::Active);
        assert_eq!(resolve_state(59, true, false, false), TickState::Active);
    }
    #[test]
    fn passivo_com_audio_idle_curto() {
        assert_eq!(resolve_state(120, true, false, false), TickState::Passive);
    }
    #[test]
    fn idle_curto_sem_audio() {
        assert_eq!(resolve_state(120, false, false, false), TickState::Idle);
    }
    #[test]
    fn away_entre_5_e_15min() {
        assert_eq!(resolve_state(600, true, false, false), TickState::Away);
        assert_eq!(resolve_state(600, false, false, false), TickState::Away);
    }
    #[test]
    fn away_uncertain_longo_com_audio() {
        assert_eq!(resolve_state(1000, true, false, false), TickState::AwayUncertain);
        assert_eq!(resolve_state(1000, false, false, false), TickState::Away);
    }
}
