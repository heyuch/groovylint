
            class MyClass {
                static def method1() {
                    File.createTempFile(null, null)
                }
                def method2() {
                    File.createTempFile(null, null, null)
                }
            }
        