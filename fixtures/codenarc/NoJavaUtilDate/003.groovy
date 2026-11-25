
            import some.other.Date

            class MyClass {
                Date myDate = new java.util.Date()      // the only violation
                def timestamp = new Date()
                Date startTime = new Date(123456789L)
            }
        