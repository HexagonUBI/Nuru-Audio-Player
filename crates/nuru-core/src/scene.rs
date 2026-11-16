use std::collections::HashMap;

use crate::model::NookRole;

#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub rain: f32,
    pub snow: f32,
    pub wind: f32,
    pub storm: f32,
    pub hearth: f32,
    pub life: f32,
    pub transit: f32,
    pub sky: String,
    pub window: String,
    pub hearth_kind: Option<String>,
}

const CAP: f32 = 1.6;

pub fn compute<'a>(layers: impl IntoIterator<Item = (&'a NookRole, f32)>, follows_volume: bool) -> Scene {
    let (mut rain, mut snow, mut wind, mut storm) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let (mut hearth, mut life, mut transit) = (0.0f32, 0.0f32, 0.0f32);
    let mut pick: HashMap<&str, (&str, f32)> = HashMap::new();

    for (role, volume) in layers {
        if role.channel == "none" {
            continue;
        }
        let strength = if follows_volume { role.weight * volume } else { role.weight };
        let channel = role.channel.as_str();
        let state = role.state.as_str();

        if channel == "weather" {
            match state {
                "rain" => rain += strength,
                "downpour" => rain += strength * 1.7,
                "storm" => {
                    storm += strength;
                    rain += strength * 0.8;
                }
                "snow" => snow += strength,
                "wind" => wind += strength,
                _ => {}
            }
            continue;
        }

        match channel {
            "hearth" => hearth = hearth.max(strength),
            "life" => life = life.max(strength),
            "transit" => transit = transit.max(strength),
            _ => {}
        }

        let replace = pick.get(channel).is_none_or(|(_, w)| strength > *w);
        if replace {
            pick.insert(channel, (state, strength));
        }
    }

    let state_of = |channel: &str| pick.get(channel).map(|(s, _)| s.to_string());
    Scene {
        rain: rain.min(CAP),
        snow: snow.min(CAP),
        wind: wind.min(CAP),
        storm: storm.min(CAP),
        hearth: hearth.min(CAP),
        life: life.min(CAP),
        transit: transit.min(CAP),
        sky: state_of("sky").unwrap_or_else(|| "dusk".into()),
        window: state_of("window").unwrap_or_else(|| "hills".into()),
        hearth_kind: state_of("hearth"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role(channel: &str, state: &str, weight: f32) -> NookRole {
        NookRole { channel: channel.into(), state: state.into(), weight }
    }

    #[test]
    fn louder_sound_wins_its_channel() {
        let city = role("window", "city", 1.0);
        let forest = role("window", "forest", 1.0);
        let s = compute([(&city, 0.3), (&forest, 0.8)], true);
        assert_eq!(s.window, "forest");
        let s = compute([(&city, 0.3), (&forest, 0.8)], false);
        assert_eq!(s.window, "city");
    }

    #[test]
    fn weather_adds_up_and_caps() {
        let storm = role("weather", "storm", 1.0);
        let pour = role("weather", "downpour", 1.0);
        let s = compute([(&storm, 1.0), (&pour, 1.0)], true);
        assert_eq!(s.storm, 1.0);
        assert_eq!(s.rain, CAP);
        assert_eq!(s.sky, "dusk");
    }
}
