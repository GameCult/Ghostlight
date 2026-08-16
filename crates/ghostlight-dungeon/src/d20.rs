use crate::domain::{OutcomeBand, RollReceipt};

pub fn capped_modifier(values: impl IntoIterator<Item = i8>) -> i8 {
    values
        .into_iter()
        .map(i16::from)
        .sum::<i16>()
        .clamp(-10, 10) as i8
}

pub fn outcome(d20: u8, modifier: i8, dc: u8) -> OutcomeBand {
    let total = i16::from(d20) + i16::from(modifier);
    let base = if total >= i16::from(dc) + 10 {
        OutcomeBand::StrongSuccess
    } else if total >= i16::from(dc) {
        OutcomeBand::Success
    } else if total >= i16::from(dc) - 5 {
        OutcomeBand::Mixed
    } else {
        OutcomeBand::Failure
    };
    shift(
        base,
        if d20 == 20 {
            1
        } else if d20 == 1 {
            -1
        } else {
            0
        },
    )
}

fn shift(band: OutcomeBand, delta: i8) -> OutcomeBand {
    use OutcomeBand::*;
    match (band, delta) {
        (Failure, 1) => Mixed,
        (Mixed, 1) => Success,
        (Success, 1) => StrongSuccess,
        (StrongSuccess, -1) => Success,
        (Success, -1) => Mixed,
        (Mixed, -1) => Failure,
        (value, _) => value,
    }
}

pub fn receipt(digest: String, d20: u8, modifier: i8, dc: u8) -> RollReceipt {
    RollReceipt {
        schema: "ghostlight.roll_receipt.v1".into(),
        assessment_digest: digest,
        d20,
        modifier_total: modifier,
        total: i16::from(d20) + i16::from(modifier),
        dc,
        outcome: outcome(d20, modifier, dc),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn caps_context_not_attributes() {
        assert_eq!(capped_modifier([8, 7, -2]), 10);
    }
    #[test]
    fn margin_bands_hold() {
        assert_eq!(outcome(10, 0, 15), OutcomeBand::Mixed);
        assert_eq!(outcome(15, 0, 15), OutcomeBand::Success);
        assert_eq!(outcome(20, 5, 15), OutcomeBand::StrongSuccess);
    }
    #[test]
    fn natural_one_only_shifts_one_band() {
        assert_eq!(outcome(1, 10, 5), OutcomeBand::Mixed);
    }
}
