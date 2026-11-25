
            class Person {
                String firstName
                String lastName
                static mapping = {
                    table 'people'
                    columns {
                        firstName column: 'First_Name'
                        lastName column: 'Last_Name'
                        firstName column: 'First_Name'
                    }
                }
            }
        