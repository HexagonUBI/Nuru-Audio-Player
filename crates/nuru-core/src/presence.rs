use std::collections::HashMap;

use crate::persist::random_u64;
use crate::scene::Scene;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mood {
    Storm,
    Downpour,
    Rain,
    Snow,
    Hearth,
    Cafe,
    City,
    Forest,
    Beach,
    Night,
    Morning,
    Noise,
    Quiet,
    Mixed,
}

fn lines(mood: Mood) -> &'static [&'static str] {
    match mood {
        Mood::Storm => &[
            "Counting seconds between the flashes",
            "Letting the storm do the talking",
            "Somewhere dry, thankfully",
            "The sky is in a mood",
        ],
        Mood::Downpour => &[
            "It is really coming down",
            "Not going outside today",
            "Watching the gutters give up",
            "Rain, with commitment",
        ],
        Mood::Rain => &[
            "Rain on the window, tea in hand",
            "Listening to the roof",
            "Grey outside, warm inside",
            "The good kind of weather",
        ],
        Mood::Snow => &[
            "Everything is quieter with snow",
            "Watching it settle",
            "Snow is falling, nobody is coming",
            "The world got a blanket",
        ],
        Mood::Hearth => &["Closest seat to the fire", "Feeding the fire", "Warm side facing in", "Nothing beats a fire"],
        Mood::Cafe => &[
            "Eavesdropping politely",
            "In the corner with a notebook",
            "Third refill, no regrets",
            "Somebody else can make the coffee",
        ],
        Mood::City => &[
            "Up above the traffic",
            "City lights, window shut",
            "Eight million people, none of them here",
            "The city hums itself to sleep",
        ],
        Mood::Forest => &[
            "Somewhere with more trees than people",
            "Out where the signal ends",
            "The forest is doing fine without us",
            "Deep in the green",
        ],
        Mood::Beach => &[
            "Toes in the imaginary sand",
            "Tide coming in",
            "The sea repeating itself, kindly",
            "Salt in the air, allegedly",
        ],
        Mood::Night => &["Long past bedtime", "On the night shift", "Crickets have the floor", "The quiet hours"],
        Mood::Morning => &[
            "Birds are up first, as usual",
            "Easing into the day",
            "Early, on purpose",
            "The morning shift",
        ],
        Mood::Noise => &[
            "Tuning the world out",
            "Static and nothing else",
            "Building a wall of nothing",
            "Focus mode, engaged",
        ],
        Mood::Quiet => &["Picking something to listen to", "Deciding on a mood", "Browsing the weather"],
        Mood::Mixed => &[
            "Building a little weather",
            "Somewhere cosy",
            "Assembling an atmosphere",
            "Making a place to be",
        ],
    }
}

pub fn mood_of(scene: &Scene, playing: usize, all_noise: bool) -> Mood {
    if playing == 0 {
        return Mood::Quiet;
    }
    if scene.storm > 0.25 {
        return Mood::Storm;
    }
    if scene.rain > 0.9 {
        return Mood::Downpour;
    }
    if scene.snow > 0.3 && scene.snow >= scene.rain {
        return Mood::Snow;
    }
    if scene.rain > 0.2 {
        return Mood::Rain;
    }
    if scene.hearth > 0.3 {
        return Mood::Hearth;
    }
    match scene.window.as_str() {
        "city" => return Mood::City,
        "forest" => return Mood::Forest,
        "beach" => return Mood::Beach,
        _ => {}
    }
    if scene.life > 0.3 {
        return Mood::Cafe;
    }
    if all_noise {
        return Mood::Noise;
    }
    match scene.sky.as_str() {
        "night" => Mood::Night,
        "morning" => Mood::Morning,
        _ => Mood::Mixed,
    }
}

#[derive(Default)]
pub struct Lines {
    chosen: HashMap<Mood, &'static str>,
}

impl Lines {
    pub fn text(&mut self, scene: &Scene, playing: usize, all_noise: bool, cozy: bool) -> (String, String) {
        let mood = mood_of(scene, playing, all_noise);
        let line = *self.chosen.entry(mood).or_insert_with(|| {
            let pool = lines(mood);
            pool[(random_u64() % pool.len() as u64) as usize]
        });
        let count = match playing {
            0 => "Nothing playing".to_string(),
            1 => "1 soundscape".to_string(),
            n => format!("{n} soundscapes"),
        };
        let details = if cozy { format!("{count}, curled up") } else { count };
        (details, line.to_string())
    }
}
