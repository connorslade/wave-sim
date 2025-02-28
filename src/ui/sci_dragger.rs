use egui::{emath::Numeric, DragValue, Ui};

pub struct SciDragValue<'a, Num: Numeric> {
    value: &'a mut Num,
    signifacant_figures: u8,
}

impl<'a, Num: Numeric> SciDragValue<'a, Num> {
    pub fn new(value: &'a mut Num) -> Self {
        Self {
            value,
            signifacant_figures: 3,
        }
    }

    pub fn show(self, ui: &mut Ui) {
        let initial_value = self.value.to_f64();
        let exponent = (initial_value.abs().log10().floor()) as i32;
        let speed = 10.0_f64.powi(exponent - (self.signifacant_figures as i32 - 1));

        ui.add(
            DragValue::new(self.value)
                .speed(speed)
                .custom_parser(|_str| {
                    // todo
                    None
                })
                .custom_formatter(|val, _| {
                    let exponent = (val.abs().log10().floor()) as i32;
                    let significand = val / f64::powi(10.0, exponent);
                    let decimal_places = self.signifacant_figures as usize - 1;

                    if exponent == 0 {
                        format!("{significand:.00$}", decimal_places)
                    } else {
                        format!(
                            "{significand:.01$}×10{}",
                            superscript_number(exponent),
                            decimal_places
                        )
                    }
                }),
        );
    }
}

fn superscript_number(mut num: i32) -> String {
    const EXPONENTS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];

    let neg = num < 0;
    num = num.abs();

    let mut out = String::new();
    while num > 0 {
        out.insert(0, EXPONENTS[num as usize % 10]);
        num /= 10;
    }

    if neg {
        out.insert(0, '¯');
    }

    out
}
