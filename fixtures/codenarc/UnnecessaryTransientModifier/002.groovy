
                class MyClass {
                    class InnerClass {
                        // class not serializable, violation occurs
                        transient String property
                    }
                }
        