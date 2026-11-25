
class MyClass {
    def processResults() {
        return new MockRule(
            compilerPhase: compilerPhase,
            applyTo: { SourceCode source ->
                assert source.astCompilerPhase == compilerPhase
                []
            }
        )
    }
    def execute() {
        def resultsProcessor = [processResults:{ results ->
            assert results == RESULTS
            resultsProcessorCalled = true
        }] as ResultsProcessor
    }
}
        