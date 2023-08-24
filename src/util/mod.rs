use bevy::time::Timer;



pub struct RepeatingLocalTimer<const TMILLIS: usize> {
    pub timer: Timer
}

impl<const TMILLIS: usize> Default for RepeatingLocalTimer<TMILLIS> {
    fn default() -> Self {
        Self { timer: Timer::from_seconds(TMILLIS as f32 / 1000., bevy::time::TimerMode::Repeating) }
    }
}

pub struct LocalTimer<const TMILLIS: usize> {
    pub timer: Timer
}

impl<const TMILLIS: usize> Default for LocalTimer<TMILLIS> {
    fn default() -> Self {
        Self { timer: Timer::from_seconds(TMILLIS as f32 / 1000., bevy::time::TimerMode::Once) }
    }
}