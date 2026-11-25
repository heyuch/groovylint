
            String getValue(boolean isActive) {
                String value = 'abc'
                if (isActive) {
                    value = 'def'
                }
                
                for (int i=0 ;i<3; i++) {
                    i = i + 3
                }

                // Multiple assignment
                def (userName, email) = ['abc', 'abc@email.com']
                def (String candyName, int count) = ['M&M', 13]

                userName = 'new_user'
                count = 99
                
                while (count < 1000) {
                    if (email) {
                        String alias
                        alias = 'joe'
                        count = count + 1
                    }
                }

                return value
            }
        