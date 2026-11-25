
            class MyClass {
                def myClosure = {
                    try {
                        doSomething()
                    } catch(MyException e) {
                        e.printStackTrace()
                    }
                    finally
                    {
                        // TODO Should do something here
                    }
                }
            }
        