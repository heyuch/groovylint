
            class MySerializableClass implements Serializable {
                // OK, class is serializable
                transient String property
            }

            class MyClass {
                class InnerClass implements Serializable {
                    // class not serializable, violation occurs
                    transient String property
                }
            }
        