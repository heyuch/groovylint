
            class Person {
                String firstName
                String lastName
                static mapping = {
                    columns {
                        firstName column: buildColumnName(1)
                        lastName column: buildColumnName(2)
                    }
                }
            }
        