import {
  Injectable,
  UnauthorizedException,
  ForbiddenException,
  BadRequestException,
  NotFoundException,
} from '@nestjs/common';
import { JwtService } from '@nestjs/jwt';
import { UsersService } from '../users/users.service';
import { MailService } from '../mail/mail.service';
import * as bcrypt from 'bcrypt';
import * as crypto from 'crypto';

@Injectable()
export class AuthService {
  constructor(
    private usersService: UsersService,
    private jwtService: JwtService,
    private mailService: MailService,
  ) {}

  async register(email: string, password: string) {
    const existing = await this.usersService.findByEmail(email);
    if (existing) throw new BadRequestException('Email already in use');

    const passwordHash = await bcrypt.hash(password, 10);
    const { token, hash, expiresAt } = this.generateVerificationToken();

    const user = await this.usersService.create({
      email,
      passwordHash,
      isVerified: false,
      verificationToken: hash,
      verificationTokenExpiresAt: expiresAt,
    });

    await this.mailService.sendVerificationEmail(user.email, token);

    return { message: 'Registration successful. Please verify your email.' };
  }

  async login(email: string, password: string) {
    const user = await this.usersService.findByEmail(email);
    if (!user || !(await bcrypt.compare(password, user.passwordHash))) {
      throw new UnauthorizedException('Invalid credentials');
    }
    if (!user.isVerified) {
      throw new ForbiddenException('Please verify your email before logging in');
    }
    return this.signToken(user.id, user.email);
  }

  async verifyEmail(token: string) {
    const hash = this.hashToken(token);
    const user = await this.usersService.findByVerificationToken(hash);

    if (!user) throw new BadRequestException('Invalid or expired verification token');
    if (user.verificationTokenExpiresAt < new Date()) {
      throw new BadRequestException('Verification token has expired');
    }

    await this.usersService.update(user.id, {
      isVerified: true,
      verificationToken: null,
      verificationTokenExpiresAt: null,
    });

    return { message: 'Email verified successfully. You can now log in.' };
  }

  async resendVerification(email: string) {
    const user = await this.usersService.findByEmail(email);
    if (!user) throw new NotFoundException('User not found');
    if (user.isVerified) throw new BadRequestException('Email is already verified');

    const { token, hash, expiresAt } = this.generateVerificationToken();
    await this.usersService.update(user.id, {
      verificationToken: hash,
      verificationTokenExpiresAt: expiresAt,
    });

    await this.mailService.sendVerificationEmail(user.email, token);
    return { message: 'Verification email resent.' };
  }

  private generateVerificationToken() {
    const token = crypto.randomBytes(32).toString('hex');
    const hash = this.hashToken(token);
    const expiresAt = new Date(Date.now() + 24 * 60 * 60 * 1000); // 24h
    return { token, hash, expiresAt };
  }

  private hashToken(token: string) {
    return crypto.createHash('sha256').update(token).digest('hex');
  }

  private signToken(userId: string, email: string) {
    return {
      access_token: this.jwtService.sign({ sub: userId, email }),
    };
  }
}
