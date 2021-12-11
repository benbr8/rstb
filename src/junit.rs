use crate::test;
use junit_report::{Duration, TestCaseBuilder, TestSuiteBuilder, ReportBuilder};

pub(crate) fn create_junit_xml() {
    let tests = test::TESTS.get().unwrap();
    let mut test_cases = Vec::new();

    for t in tests.iter().map(|obj| obj.get()) {
        let tc = match t.result.as_ref().unwrap() {
            Ok(_) => TestCaseBuilder::success(&t.name, Duration::seconds_f64(t.time_secs)),
            Err(e) => TestCaseBuilder::failure(
                &t.name,
                Duration::seconds_f64(t.time_secs),
                "failure",
                &format!("{:?}", e),
            ),
        }.build();
        test_cases.push(tc);
    }

    let test_suite = TestSuiteBuilder::new(crate::CRATE_NAME.get().unwrap())
        .add_testcases(test_cases)
        .build();
    let report = ReportBuilder::new().add_testsuite(test_suite).build();
    let file = std::fs::File::create("results.xml").unwrap();
    report.write_xml(file).unwrap();
}
