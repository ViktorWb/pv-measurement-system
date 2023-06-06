pub fn iteration(
    prev_output: f32,
    voltage: i32,
    current: i32,
    prev_voltage: i32,
    prev_current: i32,
) -> f32 {
    let dv = voltage - prev_voltage;

    println!("dv: {dv}");

    const STEP: f32 = 0.01;

    let mut output = prev_output;

    let p = voltage as i32 * current as i32;
    let p_prev = prev_voltage as i32 * prev_current as i32;
    let dp = p - p_prev;

    println!("dp: {dp}");

    if dp > 0 {
        if dv > 0 {
            output += STEP;
        } else {
            output -= STEP;
        }
    } else if dp < 0 {
        if dv < 0 {
            output += STEP;
        } else {
            output -= STEP;
        }
    }

    output.max(0.0).min(1.0)
}
