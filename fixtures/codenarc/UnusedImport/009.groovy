
            import static System.getProperties
            import static a.SomeClass.isEnabled
            import static b.OtherClass.setInstance
            class ABC {
                def foo() {
                    properties.getProperty("foo")
                }
                def bar() {
                    if (enabled) {
                        instance = this
                    }
                }
            }
        