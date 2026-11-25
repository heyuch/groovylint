
            def x = 42
            def s = "some string ${-> x}"
            def s = "some string ${x} ${-> x}"
        