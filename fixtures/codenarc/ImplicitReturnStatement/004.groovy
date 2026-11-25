
            class MyClass {
                int execute() {
                    try {
                        return executeTask(new Date())
                    }
                    catch(AnalyzerException e) {
                        throw new Exception('Task failed: ' + e.getMessage(), e)
                    }
                }
            }
        