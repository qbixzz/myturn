use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ColorRule {
    pub min:   f64,
    pub max:   f64,
    pub hex:   String,
    pub label: String,
}

/// Returns the hex color for the first ColorRule whose [min, max] contains
/// `percent`. Falls back to last rule if none match (floating-point edge at 100.0).
pub fn color_for_percent<'a>(percent: f64, rules: &'a [ColorRule]) -> &'a str {
    for rule in rules {
        if percent >= rule.min && percent <= rule.max {
            return &rule.hex;
        }
    }
    rules.last().map(|r| r.hex.as_str()).unwrap_or("#FFFFFF")
}

pub fn default_color_rules() -> Vec<ColorRule> {
    vec![
        ColorRule { min: 0.0,  max: 44.0,  hex: "#2ECC71".into(), label: "Normal".into()   },
        ColorRule { min: 45.0, max: 64.0,  hex: "#F1C40F".into(), label: "Moderate".into() },
        ColorRule { min: 65.0, max: 84.0,  hex: "#E67E22".into(), label: "Warning".into()  },
        ColorRule { min: 85.0, max: 100.0, hex: "#E74C3C".into(), label: "Critical".into() },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> Vec<ColorRule> { default_color_rules() }

    #[test]
    fn normal_usage_returns_green() {
        assert_eq!(color_for_percent(42.0, &rules()), "#2ECC71");
    }

    #[test]
    fn boundary_44_is_still_normal() {
        assert_eq!(color_for_percent(44.0, &rules()), "#2ECC71");
    }

    #[test]
    fn moderate_45_returns_yellow() {
        assert_eq!(color_for_percent(45.0, &rules()), "#F1C40F");
    }

    #[test]
    fn warning_65_returns_orange() {
        assert_eq!(color_for_percent(65.0, &rules()), "#E67E22");
    }

    #[test]
    fn critical_85_returns_red() {
        assert_eq!(color_for_percent(85.0, &rules()), "#E74C3C");
    }

    #[test]
    fn critical_100_returns_red() {
        assert_eq!(color_for_percent(100.0, &rules()), "#E74C3C");
    }
}
