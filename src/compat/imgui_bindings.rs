use crate::compat::inputs;

pub fn input_rs<T: AsRef<str>>(label: T, buf: &mut String) {
    inputs::InputText::new(label, buf).build();
}
