
            @SuppressWarnings('CodeNarc')
            def relativePath = filePath.substring(path.length())

            @SuppressWarnings('ClassForName')
            def driver = Class.forName(driverName)

            @SuppressWarnings('Other')
            String name =  myName + 'abc'

            @SuppressWarnings('Other')
            def otherName = "abc" + "***"

            @SuppressWarnings('Other')
            void method1(String name, int count = 99, long id = 1) { }
            