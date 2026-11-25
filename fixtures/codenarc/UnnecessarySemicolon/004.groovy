
            def var = 'yo yo yo'
            def javascript = """
                // this next semicolon is ignored
                window.alert($var);
            // this next semicolon is not ignored!
            """;
            