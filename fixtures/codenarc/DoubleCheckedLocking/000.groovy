
            def result = object
            if (result == null) {
                synchronized(this) {
                    result = object
                    if (result == null)
                        object = result = createObject()
                }
            }

            // and a better solution for a singleton:
            class myClass  {
                private static class ObjectHolder {
                   public static Object object = createObject()
                }
                public static Object getObject() {
                    return ObjectHolder.object
                }
            }

            if (object1 == null) {
                synchronized(this) {
                    if (object1 == null) {
                        object2 = createObject()
                    }
                }
            }

            if (object == 5) {
                synchronized(this) {
                    if (object == null) {
                        object = createObject()
                    }
                }
            }

            if (object == 5) {
                synchronized(this) {
                    if (5 == object) {
                        object = createObject()
                    }
                }
            }

            if (object == 5) {
                synchronized(this) {
                    if (object == null) {
                        x = createObject()
                    }
                }
            }

            if (object == 5) {
                synchronized(this) {
                    if (object == null) {
                        doSomething()
                        object = createObject()
                    }
                }
            }

            if (object == 5) {
                synchronized(this) {
                    doSomething()
                    if (object == null) {
                        object = createObject()
                    }
                }
            }

            if (object == 5) {
                doSomething()
                synchronized(this) {
                    if (object == null) {
                        object = createObject()
                    }
                }
            }
        