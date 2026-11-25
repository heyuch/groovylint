
            class Person {
                String firstName
                String lastName
                static mapping = {
                    table 'people'
                    firstName column: 'First_Name'
                    lastName column: 'Last_Name'
                    firstName column: 'First_Name'
                    table 'people2'
                }
            }
        