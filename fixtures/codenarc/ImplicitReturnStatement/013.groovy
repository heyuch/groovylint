
            class MyClass {
                String someMethod() {
                    synchronized (someObject) {
                        String result = doSomething();
                        return result;
                    }
                }
            }
        