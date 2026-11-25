
class MyClass {
    protected createRunner = { new ProcessRunner() }
    protected createOtherRunner = {
        if (excludeBaseline) {
            LOG.info("Loading baseline violations from [$excludeBaseline]")
            def resource = resourceFactory.getResource(excludeBaseline)
            def resultsProcessor = new BaselineResultsProcessor(resource)
            return new CodeNarcRunner(resultsProcessor:resultsProcessor)
        }
        return new CodeNarcRunner()
    }
}
        