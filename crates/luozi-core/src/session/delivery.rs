//! Map target validation to insert / clipboard / discard (spec §5.3).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetValidation {
    SameTarget,
    Changed,
    Unsupported,
    Secure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryAction {
    Insert,
    Clipboard,
    Discard,
}

pub fn route_delivery(validation: TargetValidation) -> DeliveryAction {
    match validation {
        TargetValidation::SameTarget => DeliveryAction::Insert,
        TargetValidation::Changed | TargetValidation::Unsupported => DeliveryAction::Clipboard,
        TargetValidation::Secure => DeliveryAction::Discard,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_target_inserts() {
        assert_eq!(
            route_delivery(TargetValidation::SameTarget),
            DeliveryAction::Insert
        );
    }

    #[test]
    fn changed_and_unsupported_use_clipboard() {
        assert_eq!(
            route_delivery(TargetValidation::Changed),
            DeliveryAction::Clipboard
        );
        assert_eq!(
            route_delivery(TargetValidation::Unsupported),
            DeliveryAction::Clipboard
        );
    }

    #[test]
    fn secure_discards() {
        assert_eq!(
            route_delivery(TargetValidation::Secure),
            DeliveryAction::Discard
        );
    }
}
