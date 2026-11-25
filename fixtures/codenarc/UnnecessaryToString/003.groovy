
            def string = "some string${123L.toString()} or ${123} or ${'ABC'} or ${new Date().toString()}"
            def string2 = """
                 processing: ${123L.toString()}
                 processing: ${new Date().toString()}
                """
        