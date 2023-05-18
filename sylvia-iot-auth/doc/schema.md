# Schema - Auth

## User

    user: {
        userId: string,                 // (unique) user ID
        account: string,                // (unique) user account
        createdAt: Date,                // creation time
        modifiedAt: Date,               // modification time
        verifiedAt: Date | null,        // verification time
        expiredAt: Date | null,         // expiration time to prevent malicious attack
        disabledAt: Date | null,        // mark this account disabled
        roles: object,                  // roles with booleans
        password: string,               // hashed password
        salt: string,                   // salt for password hash
        name: string,                   // display name
        info: object                    // other information such as address, telephone number, ...
    }

## Client

    client: {
        id: string,                     // (unique) client ID
        createdAt: Date,                // creation time
        modifiedAt: Date,               // modification time
        clientSecret: string | null,    // client secret
        redirectUris: string[],         // allowed redirect URIs
        scopes: string[],               // allowed scopes.
        userId: string,                 // developer's user ID corresponding to the `user` collection
        name: string,                   // client name
        imageUrl: string | null         // image URL
    }

## Login Session

    loginSession: {
        sessionId: string,              // (unique) session ID
        expiresAt: Date,                // expiration date time
        userId: string                  // associated user ID corresponding to `users` collection
    }

## Authorization Code

    authorizationCode: {
        code: string,                   // (unique) authorization code
        expiresAt: Date,                // expiration date time
        redirectUri: string,            // allowed redirect URIs
        scope: string | null,           // authorized scope(s)
        clientId: string,               // client ID corresponding to `client` collection
        userId: string                  // associated user ID corresponding to `users` collection
    }

## Access Token

    accessToken: {
        accessToken: string,            // (unique) access token
        refreshToken: string,           // (unique) refresh token corresponding to `refreshToken` collection
        expiresAt: Date,                // expiration time
        scope: string | null,           // authorized scope(s)
        redirectUri: string,            // the redirect URI
        clientId: string,               // client ID corresponding to `client` collection
        userId: string                  // associated user ID corresponding to `users` collection
    }

## Refresh Token

    refreshToken: {
        refreshToken: string,           // (unique) refresh token
        expiresAt: Date,                // expiration time
        scope: string | null,           // authorized scope(s)
        redirectUri: string,            // the redirect URI
        clientId: string,               // client ID corresponding to `client` collection
        userId: string                  // associated user ID corresponding to `users` collection
    }
