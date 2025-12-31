//use github-bot::multiply;

#[derive(::core::default::Default)]
struct World {
    inputs: (i32, i32),
    result: i32,
}

// TODO: Replace with real tests
fn main() -> std::process::ExitCode {
    ::litmus::Runner::new()
        .feature(
            ::litmus::Feature::new()
                .description("Arithmetic Operations")
                .scenario(
                    ::litmus::Scenario::<World>::new()
                        .description("Multiply two numbers")
                        .given("the numbers 10 and 5", |w| w.inputs = (10, 5))
                        .when("I multiply them", |w| {
                            let (a, b) = w.inputs;
                            w.result = a * b;
                        })
                        .then("the result should be 50", |w| {
                            ::litmus::assert!(w.result == 50, "10 * 5 = 50")
                        }),
                ),
        )
        .run()
}
