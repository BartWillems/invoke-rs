use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use lingua::Language;

#[derive(Clone)]
pub struct LanguageDetector {
    enabled: Arc<AtomicBool>,
    detector: Arc<lingua::LanguageDetector>,
}

impl LanguageDetector {
    pub fn new(default_state: bool) -> Self {
        Self {
            enabled: AtomicBool::from(default_state).into(),
            detector: lingua::LanguageDetectorBuilder::from_languages(&[
                Language::French,
                Language::English,
                Language::Dutch,
            ])
            .build()
            .into(),
        }
    }

    pub fn has_french(&self, msg: String) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            log::debug!("french detector disabled");
            return false;
        }

        let Some(language) = self.detector.detect_language_of(msg) else {
            log::debug!("no language found");
            return false;
        };

        match language {
            Language::Dutch | Language::English => false,
            Language::French => true,
        }
    }

    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }
}
