
            class MyClass {
                def myClosure = {
                    try {
                    } catch(MyException ignore) {
                    }

                    try {
                    } catch(MyException ignored) {
                    }
                }
            }
        