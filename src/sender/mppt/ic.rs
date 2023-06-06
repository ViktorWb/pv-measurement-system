pub fn iteration(
    prev_output: f32,
    voltage: i16,
    current: i16,
    prev_voltage: i16,
    prev_current: i16,
) -> f32 {
    let dv = voltage - prev_voltage;
    let di = current - prev_current;

    println!("dv: {dv}");
    println!("di: {di}");

    const STEP: f32 = 0.01;

    let mut output = prev_output;

    if dv == 0 {
        if di != 0 {
            if di > 0 {
                // öka spänningen
                output += STEP;
                println!("A");
            } else {
                // minska spänningen
                output -= STEP;
                println!("B");
            }
        }
    } else {
        let test = current as i32 + di as i32 * voltage as i32 / dv as i32;
        println!("test: {test}");
        if test != 0 {
            if test > 0 {
                // öka spänningen
                output += STEP;
                println!("C");
            } else {
                // minska spänningen
                output -= STEP;
                println!("D");
            }
        }
    }

    output.max(0.0).min(1.0)
}
