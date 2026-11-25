
            def binding = [*: scriptConfig.parameters]

            def params = [:]
            to(page, *:params)
        