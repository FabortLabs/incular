/// Platform adapters may translate these stable values to native IME hints.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextInputType {
    #[default]
    Text,
    Multiline,
    /// A numeric keyboard; see [`TextInputType::number_with_options`].
    Number {
        signed: bool,
        decimal: bool,
    },
    Phone,
    Datetime,
    EmailAddress,
    Url,
    VisiblePassword,
    Name,
    StreetAddress,
    None,
}

impl TextInputType {
    /// Flutter's `TextInputType.number`: integers only, no sign.
    pub const NUMBER: Self = Self::Number {
        signed: false,
        decimal: false,
    };

    /// Flutter's `TextInputType.numberWithOptions`.
    #[must_use]
    pub const fn number_with_options(signed: bool, decimal: bool) -> Self {
        Self::Number { signed, decimal }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextInputAction {
    #[default]
    Unspecified,
    None,
    Done,
    Go,
    Search,
    Send,
    Next,
    Previous,
    Continue,
    Join,
    Route,
    EmergencyCall,
    Newline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DayPeriod {
    #[default]
    Am,
    Pm,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimeOfDayFormat {
    #[default]
    HH_colon_mm,
    HH_dot_mm,
    frenchCanadian,
    a_space_h_colon_mm,
    H_colon_mm,
    h_colon_mm_space_a,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HourFormat {
    HH,
    H,
    #[default]
    h,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimePickerEntryMode {
    #[default]
    Dial,
    Input,
    DialOnly,
    InputOnly,
}
