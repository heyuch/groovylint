
            interface FirstInterface {
                void run(String value)
            }

            interface SecondInterface {
                void run(String first, String second)
            }

            class InvalidClass implements Runnable, FirstInterface, SecondInterface {
                void run(String first = "", String second = "") {
                }
            }
        