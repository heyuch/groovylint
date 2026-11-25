
            // these usages are OK
            Integer d = 5

            def myfun() {
                def a = "foo"
                Integer b = 6
            }

            class MyCorrectClass {
                private Integer b = 6
                public static final VALUE = 1234
            }
        