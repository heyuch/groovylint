
            def sourceAnalyzer = [analyze: { rs -> ruleSet = rs; RESULTS }, getSourceDirectories: { SOURCE_DIRS }] as SourceAnalyzer
        